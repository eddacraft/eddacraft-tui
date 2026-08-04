<!--
APS Module: Continuous Improvement Backlog
==========================================
Standing intake for concrete improvement work identified across the project.
This module intentionally remains active while the project is active.
-->

# Continuous Improvement Backlog

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| CIB | —     | In Progress | 191/272  |

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

- **Status:** Released/Shipped via v0.6.2-beta (e7478b0a · 2026-05-12). Merged 2026-05-12 via PR #1453
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3091
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

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21). Merged 2026-05-21 via PR #1817 (issue #1797)
- **Summary:** The planless `anvil check` dispatcher now routes through the same
  `.anvilrc#checks` reader as `anvil gate` and runs the planless-eligible set
  (`secret-detection` + `antipattern-scan`), so the marquee `anvil check
  src/smelly.ts` demo no longer silently passes a hardcoded `sk-…` key
  (`crates/anvil-cli/src/commands/check.rs`, `PLANLESS_ELIGIBLE_CHECKS`).

### CIB-009: `anvil audit` and `anvil gate` disagree on the same repo

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21). Merged 2026-05-21 via PR #1814 (issue #1798)
- **Summary:** `anvil audit` now runs the canonical `secret-detection` check
  over the same tree as `anvil gate` (with gate-aligned file extensions), so the
  two can no longer disagree — audit no longer reports "0 issues" on a repo with
  a planted key (`crates/anvil-cli/src/commands/audit.rs`). Sibling of CIB-008.

### CIB-010: `anvil watch` first-scan emits a wall of `public-api-expansion` against existing symbols

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21). Merged 2026-05-21 via PR #1816 (issue #1802; behaviour fixed by WATCHUX-001, PR #1816 adds the regression test)
- **Summary:** On a never-baselined repo, the initial graph is now treated as
  the baseline (behaviour fixed by WATCHUX-001 — only post-scan modifications go
  through the policy engine), so `anvil watch`'s first scan no longer flags every
  pre-existing public symbol as `public-api-expansion`. PR #1816 added the
  multi-file regression test `audit_1802_multi_file_initial_scan_emits_no_public_api_violations`
  in `crates/anvil-kernel/src/watch.rs`.

### CIB-011: `anvil gate -p ai` fails strict-mode checks on missing configs without next-step guidance

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21). Merged via PR [#1818](https://github.com/eddacraft/anvil-001/pull/1818) (merged 2026-05-21 at `acc4db6f`)
- **Summary:** On a fresh repo, `anvil gate -p ai` no longer FAILs purely
  because config files don't exist yet — missing-config is info-level and the
  score grades against available checks, with a `next:` hint (issue #1803,
  new-user journey audit finding #9).

### CIB-012: `anvil check --staged` errors with "`--changed` required"

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21). Merged via PR [#1813](https://github.com/eddacraft/anvil-001/pull/1813) (merged 2026-05-21 at `ce0bd32b`)
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

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-26 via PR #1995
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-03 via PR #2270
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

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-26 via PR #1983
- **Summary:** `eval::run` carries `#[tracing::instrument]` (policy/query span)
  and a `debug!` summary (policy_bytes / input_bytes / eval_ms / findings /
  exit_code), with `warn!`s on the gate-relevant failure paths and on engine
  abnormal conditions (caught panic, poisoned lock). Surfaces under
  `ANVIL_LOG=debug` via `anvil-observability` (JSON to stdout). Added `tracing`
  to the policy-engine crate (already in the binary tree; no new crates).

### CIB-018: `catch_unwind` at the policy-engine facade boundary

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-26 via PR #1980
- **Summary:** `Engine::guard` wraps every regorus call in
  `catch_unwind(AssertUnwindSafe(..))`, converting a panic on an adversarial
  policy into `EngineError::Regorus` + poisoning the engine, so `anvil policy
  eval` returns a structured error/non-zero exit instead of aborting. Required
  flipping the CLI from `panic = "abort"` to `"unwind"` (**ADR-051**) — without
  it `catch_unwind` is a no-op in the shipped binary.

### CIB-019: Surface Go OPA stderr in the parity gate

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-26 via PR #1990
- **Summary:** `scripts/bench-vs-go-opa.sh` now captures `opa bench` stderr to a
  temp file (trap-cleaned) and surfaces it before the `require_pos_num` bail, so
  an OPA error (parse failure, crash, version skew) reaches the operator instead
  of a bare "no positive measurement". `require_pos_num` gained an optional 4th
  arg pointing at the stderr file. New fixture test `bench-vs-go-opa.test.sh`
  stubs `opa` + the harness and asserts both the happy-path `GATE: PASS` and the
  opa-error path (exit 2, OPA's text surfaced); wired into CI script-fixtures.
  From the POLENG full council (operations + adversarial seats), 2026-05-25.

### CIB-020: Release-prep must refresh version-embedding TUI snapshots

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-25 via PR #1961
- **Summary:** anvil-tui's shell watermark renders a fixed `X.Y.Z` placeholder
  in test builds (`VERSION` is `CARGO_PKG_VERSION` under `cfg(not(test))`, the
  placeholder under `cfg(test)`), so ~38 snapshots are version-agnostic and a
  release version bump no longer reddens `main`. `version_matches_workspace` →
  `production_watermark_uses_cargo_pkg_version`. Surfaced reactively by PR #1959
  during the POLENG-009 rebase.

### CIB-021: Append-only CI log should not produce merge conflicts

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-26 via PR #1967
- **Summary:** `plans/reviews/continuous-improvement-log.md` is `merge=union` in
  `.gitattributes`, so concurrent appends from parallel agents/worktrees merge
  without conflict markers (reuses the witness-NDJSON-log pattern). Entry
  convention requires a trailing blank line so unioned entries don't abut.

### CIB-022: Derive APS index progress counts instead of hand-editing

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-26 via PR #1969
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

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-26 via PR #1987
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

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-29 via PR #2063
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-03 via PR #2271
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

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-29 via PR #2059
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-27 via PR #2967 (PR-side `-D warnings` gate; publish-side `--all-features` gate merged 2026-06-16 via PR #2682)
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

- **Status:** Released/Shipped via v0.7.3-beta (8bfd48c4 · 2026-05-31). Merged 2026-05-30 via PR
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-16 via PR #2684
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-02 via PR #2241
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-03 via PR #2295
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-03 via this PR
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-02 via PR #2221
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-16 via PR #2679
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-16 via PR #2679
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-08 via PR #2440
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2474
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2475
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3078
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3080
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2481
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2493
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2496
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2485
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2512
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2515
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2873
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2858
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-10 via PR #2526
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-11 via PR #2529
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-11 via PR #2528
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

- **Status:** Released/Shipped via v0.8.0-beta (e2db4026 · 2026-06-11). Merged 2026-06-11 via PR #2530
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

- **Status:** Released/Shipped via v0.8.1-beta (2a3cfafb · 2026-06-11). Merged 2026-06-11 via PR #2539 (owner decision: target
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-11 via PR #2566 (deployed + verified live
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-12 via PR #2568
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-12 via PR #2569
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-12 via PR #2570
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-13 via PR #2577
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-14 via PR #2597 (Phase A) + PR #2611 (Phase B)
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2859
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-11 via PR #3282 — shipped the cumulative value
  scoreboard on `anvil insights` (witness-chain totals plus 30/90-day
  windows, save-time protection counts over the sidecar's retained window),
  `anvil insights --share` (deterministic, self-contained, redacted HTML
  scorecard) and the `anvil.insights.v2` JSON document under
  `--cumulative --json` (default `--json` stays v1); redaction proven by
  marker-seeded fixtures. Promoted into the JOURNEY release cut on
  2026-07-11; the privacy/redaction contract was the readiness gate.
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
- **Coordinates with:** JOURNEY-004; witness-chain / provenance surface;
  CIB-074 (the human-readable audit report shares the redaction + rendering
  concern); CIB-190 (healthy repeat-start value line consumes only trustworthy
  local aggregates).
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

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-22 via PR #3377 — preflight `--pre-prepare --version`
  mode and runbook path landed; status reconciled 2026-07-31 after validation
  confirmed the expected outcome (tests green on main).
- **Intent:** the release flow's `preflight.sh` runs a `cargo-version` gate
  (`require_workspace_version_match`) that fails when the workspace version
  equals the latest release tag — treating it as "the engineer forgot to
  bump". But `prepare.sh`, which performs the version bump, runs _after_
  `preflight.sh`. On a hotfix cut from a tagged release the workspace version
  legitimately still equals the latest tag at preflight time, so the gate
  aborts the release before `prepare` ever gets to bump it — a chicken-and-egg
  ordering wall.
- **Expected Outcome:** `preflight.sh --pre-prepare --version <candidate>`
  validates the planned candidate version and all ordinary release gates while
  allowing the source workspace to equal the latest tag until `prepare.sh`
  performs its owned bump. Default preflight behaviour remains strict, so a
  genuinely forgotten bump still fails outside the explicit release path; the
  release runbook documents the required mode.
- **Files:** `scripts/release/preflight.sh` (cargo-version gate, ~L195–325),
  `scripts/release/prepare.sh` (bump stage — ordering reference),
  `docs/runbooks/release-runbook.md`.
- **Validation:** `bash scripts/release/_test/preflight.test.sh`; cover the
  explicit pre-prepare path, the planned-version mismatch, and unchanged strict
  default behaviour.
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via PR #2717
- **Intent:** POLENG-007 shipped `anvil policy eval --json` preview-gated, with
  the wire shape explicitly _not_ a stable contract. The
  [eval-harness-integration](../archive/modules/eval-harness-integration.aps.md) module (EVAL) is
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
  `plans/archive/modules/eval-harness-integration.aps.md`.
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via PR #2815
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via PR #2823
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2852
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2852
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2852
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2852
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2852
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2870
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2865
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2887
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3083
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-22 via PR #2884
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-24 via PR #2903
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-25 via PR #2912
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3084
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3082
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

- **Status:** Done 2026-07-30 — superseded by CIB-211 and MLP2-028 after
  retirement of the unsupported Node CLI framing.
- **Supersession:** CIB-211 now owns supported driver-client authentication and
  trusted-config ACL work; MLP2-028 already owns Windows peer-PID lineage. This
  closure does not claim those gaps were implemented.
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
- **Coordinates with:** CIB-100, CIB-106, CIB-211, and MLP2-028.
- **Confidence:** medium — behaviour is clear, but reliable Windows validation is
  the gating risk.

### CIB-115: Centralise workspace path containment for TS adapters and APS loaders

- **Status:** Done 2026-07-30 — superseded by CIB-212 and CIB-213 after the
  owner retired the obsolete Node CLI framing.
- **Supersession:** CIB-212 owns APS-loader containment and CIB-213 owns
  runtime-cache index containment. This closure does not retire the listed
  release surfaces or claim that their hardening was implemented.
- **Decomposition note (2026-07-02):** held back from the
  oversized-item split on purpose — every target directory sits in the
  retiring JS/TS `packages/` tree, so filing per-consumer children before
  the owner decides invest-vs-retire would allocate ids for work that may
  never run. The supported work is now split into CIB-212 and CIB-213.
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
- **Coordinates with:** CIB-108 (policy eval trust boundary), CIB-212, CIB-213,
  and APS parser/validator governance.
- **Confidence:** high — root cause is duplicated validation; broad file touch
  warrants careful staged tests.

### CIB-116: Redact provenance and debug secrets before persistence or logs

- **Status:** Done 2026-07-30 — superseded by CIB-214 and CIB-215 after archival
  of the unsupported Node admin CLI.
- **Supersession:** CIB-214 owns live API debug redaction and CIB-215 owns
  provenance credential redaction. This closure does not claim those proposals
  were implemented.
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
- **Coordinates with:** CIB-214, CIB-215, provenance/Git AI standard outputs, and
  the admin CLI operator runbook.
- **Confidence:** high — secret shapes are concrete; compatibility risk is limited
  to documented input channels and provenance schema expectations.

### CIB-117: Fence TS runtime and APS state transitions against lost updates

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3087
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3088
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3086
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3077
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
- **Coordinates with:** CIB-111 for shared API primitives; CIB-214 for
  structured debug redaction.
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-30 via PR #3011
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-01 via PR #3021 (full 5-reviewer council; blockers
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-01 via PR #3024 — `crates/anvil-witness/tests/
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-01 via PR #3026 (full 5-reviewer council; blockers
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3076
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

- **Status:** Done
- **Summary:** Already covered — `cap_threads_is_half_cores_minimum_one` in
  `crates/anvil-rayon-init/src/lib.rs` pins `cap_threads` across representative
  core counts (predates this item, commit `1b1fecdb2`). Reconciled 2026-07-05
  after a readiness-gate check found the item stale relative to current main.

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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-19 via commit `a4b3a31c5`
- **Summary:** Already fixed — `scripts/dogfood/external-fp/classify.py`'s
  `source_line()` rejects any path resolving outside the repo root (guarded
  `relative_to`), and `build()` creates the worksheet output directory when
  missing. Landed before the 2026-07-02 clawpatch triage that (re-)identified
  it. Reconciled 2026-07-05 after a readiness-gate check found the item stale
  relative to current main.

### CIB-132: Precise SSL detection in `admin-key-manage`

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-02 via PR #3079
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3185
- **Summary:** Threaded `project_writes_gated` into the canonical
  `first_week_insights_hint(root, now, gated)`, which now returns `None` with no
  read and no write when gated. Updated the `status` and `watch` call sites to
  pass `install_root::project_writes_gated()` and dropped INSIGHTS-005's
  `welcome`-specific `welcome_insights_hint` wrapper so all three nudge surfaces
  (`status`, `watch`, `welcome`) honour the gate uniformly — a side-by-side
  install under a gated `ANVIL_HOME` no longer burns the real project's
  once-per-week marker or writes `.anvil/insights-hint.json` into it
  (DISTRIB-006 / ADR-060). Added a gated-root test per surface.

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

- **Status:** Done 2026-07-03 — delivered by CIB-136 via PR #3097 (DNS stack-trust gating + untrusted-stack zero-resource test landed there)
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3097
- **Intent:** `infra.yml`'s `preview` job (same-repo `pull_request`, `infra.yml:62-64`) checks out the PR branch, runs `pnpm install --frozen-lockfile` from PR-controlled `pnpm-lock.yaml`/`package.json` (`infra.yml:69-82`), authenticates to Azure with `secrets.ARM_CLIENT_ID`/`ARM_CLIENT_SECRET`/... (`infra.yml:84-90`), fetches the production `vercel-token` Key Vault secret via a raw `az keyvault secret show` call inside the workflow itself (`infra.yml:92-96` — this bypasses `infra/src/keyvault.ts` entirely), then runs `pulumi/actions@...` "Pulumi Preview" (`infra.yml:98-116`) with `ARM_*`, `VERCEL_API_TOKEN`, `PULUMI_CONFIG_PASSPHRASE`, and `AZURE_STORAGE_KEY` all exported as step env against the PR's own checked-out `infra/` Pulumi program. CIB-119 (PR #3086, merged) only edits `infra/src/*.ts` stack gating and `infra/scripts/admin-key-manage.mjs`; it never touches this workflow file (confirmed via `git show edb8895fb --stat` — no `infra.yml` in the diff), so this ordering gap is unaddressed on main.
- **Expected Outcome:** The PR preview path no longer exposes Azure Key Vault or Pulumi secrets to PR-controlled `infra/` code; the Vercel token fetch is either removed from the raw workflow step or routed through the already-gated `infra/src/keyvault.ts` path so untrusted-stack previews get the `<untrusted-stack-secret:name>` marker instead of a live secret.
- **Files:** `.github/workflows/infra.yml`.
- **Validation:** A workflow dry-run (or targeted Pulumi preview test) showing the preview job's Pulumi Preview step no longer receives a live Vercel/Key-Vault token when run against the dev/preview stack, plus static review confirming no `az keyvault secret show` call reads the production vault outside the trusted path.
- **Identified From:** Split from CIB-110 (deepsec sweep 20260629190245); decomposition readiness pass 2026-07-02.
- **Coordinates with:** CIB-119 (infra/src stack-trust gating, PR #3086, merged — its own "Coordinates with" note already defers workflow-side secret exposure to CIB-110; this item is that deferred half); CIB-139 (release/signing tag trust, related but separate secrets path).
- **Confidence:** high — the exposure is visible directly in the current workflow YAML, and no fork check protects internal (non-fork) PR branches from it.

### CIB-137: Make required-check classification tamper-resistant to PR-controlled code

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3098
- **Intent:** `detect-changes`'s classification is computed entirely from code checked out at the PR's own head (`ci.yml:52-59` checks out the default `pull_request` ref, then `uses: ./.github/actions/detect-changes`, itself PR-controlled). The composite action shells out to `scripts/ci/classify-changes.sh` (`.github/actions/detect-changes/action.yml:182`) and derives every `*-required` gate (`lint-required`, `typecheck-required`, `unit-tests-required`, `dependency-audit-required`, etc., `action.yml:227-272`) purely from that PR-editable script's output. A PR that edits `.github/actions/detect-changes/action.yml` or `scripts/ci/classify-changes.sh` to always report `docs-only`/no required checks makes `ci.yml`'s per-job skip conditions (e.g. `ci.yml:449-457`, `ci.yml:529-536`) short-circuit to a passing filler conclusion for every required-check name, without lint/typecheck/tests/dependency-audit ever running on that same PR.
- **Expected Outcome:** The composite action and script that decide which required checks run are resolved from a trusted ref (PR base SHA or `main`) rather than the PR's own head, so PR-controlled edits to the classifier cannot suppress required checks on that same PR.
- **Files:** `.github/actions/detect-changes/action.yml`, `scripts/ci/classify-changes.sh`.
- **Validation:** A test PR that edits the classifier to force `docs-only=true` while touching source files still runs the full required-check set (or the workflow fails closed), verified via a dry-run/fixture harness plus static review of the classifier's checkout ref.
- **Identified From:** Split from CIB-110 (deepsec sweep 20260629190245); decomposition readiness pass 2026-07-02.
- **Coordinates with:** CIB-038 (Lint & Format / Type Check filler-job consolidation, which consumes this classifier's outputs); CIB-136/CIB-138/CIB-139 siblings.
- **Confidence:** high — the spoof path is a direct read of currently-committed workflow logic, not a hypothetical.

### CIB-138: Restrict bench-nightly self-hosted runner to trusted refs

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3094
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3096
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

- **Status:** Proposed (rotation half landed 2026-07-18 on `test/clawpatch`;
  entitlement-default half still product-gated)
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
- **Progress (2026-07-18 clawpatch):** rotation half implemented as
  `consumeAndRotateRefreshToken` (single data-modifying CTE) +
  `mintRotatedSession` on `/session/refresh`. Closes the clawpatch high
  "Refresh-token race can mint a live session after family revocation".
  Unit coverage in `auth-session` / `session` / `queries` tests.
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3112
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3095
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3113
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
  overlap before scoping work), MLP2-026 spec.
- **Confidence:** low — this child needs a decision-log entry before any
  code changes; treat the parent's "wire-protocol change" framing as
  provisional pending that decision.

### CIB-148: Normalise source/target paths in anvil_query_boundary before policy evaluation

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3111
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3117
- **Summary:** `Allowlist` mode now fails closed to the operator allow entries
  only — no implicit primary is admitted at all. The first pass merely relocated
  the bypass (sourcing the primary from the connection's `RegisterSession`
  worktree), but that worktree is equally client-supplied: the daemon verifies
  *who* the peer is (PID lineage), never that the *path* should be admitted, so a
  same-uid client could still register then name an arbitrary root. No genuinely
  daemon-attested worktree source exists, so the verified-primary mechanism was
  dropped; `set_originating_session` now records the session for telemetry
  correlation only. `Open`-mode first-touch adoption is unchanged
  (`crates/anvil-intercept/src/{confinement.rs,ipc.rs,save_time.rs}`).

### CIB-150: Verify the wire `agent_tag` claim before honouring durable membership

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3116
- **Summary:** Closed the trust-boundary gap where any same-uid IPC client
  could mint an `AgentTag` claiming `activation-spine` durable membership and
  consume the registered-worktree budget. A durable `RegisterSession` claim is
  now honoured only when the authenticated peer is independently authorised
  (mirroring `verify_lineage_claim`), gated on trustworthy peer-exe reads;
  otherwise it is safely downgraded to an ordinary capped, TTL-bound session
  rather than rejected. Follow-up peer-exe hardening filed as CIB-160. Touches
  `crates/anvil-intercept/src/{registry.rs,ipc.rs}`.

### CIB-151: Verify on-disk state before trusting a wire-declared delete/rename

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3115
- **Summary:** `validate_paths` now reads on-disk bytes and antipattern-scans a
  path even when the client declares it `Deleted`/`Renamed`, so long as the
  path still resolves to readable content; only an actually-vanished path is
  treated as content-free, closing the `change_has_bytes` bypass. Oversized
  paths preserve their delete/rename `StaleReason`. Follow-up CIB-159 filed for
  the rename-source scan gap.

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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3188
- **Summary:** Live, lineage-verified sessions now record the authenticated
  peer identity (uid/pid/starttime) at registration; their `Heartbeat` and
  `UnregisterSession` calls are rejected when the calling peer does not match
  the recorded launcher owner (mirroring the `ReportProcess` peer-pid
  contract), independent of the telemetry-subscriber binding. The binding is
  **scoped to sessions that carry a verified lineage claim**
  (`launcher_pid == Some`): the CIB-153 threat model is a same-uid neighbour
  guessing a *live* session's id. Sessions registered without a verified
  lineage claim (`launcher_pid == None`) are unaffected and keep the
  pre-existing same-uid-socket authorization boundary — this covers durable
  worktree memberships (`anvil workspace register`/`unregister`, separate
  one-shot CLI processes with no single owner pid, per ADR-094 decision-3) and
  Windows sessions (whose peer is same-SID authenticated, but whose client PID
  is not yet threaded into lifecycle ownership; MLP2-028). Dispatch-level
  tests inject peer A/B credentials to prove cross-peer denial and same-peer
  acceptance for lineage-bearing sessions, and prove no-lineage durable
  memberships remain heartbeat/unregisterable from any same-uid peer
  (`crates/anvil-intercept/src/{registry.rs,ipc.rs}`).

### CIB-154: Cap the number of workspace roots a connection may admit

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3187
- **Summary:** Added a `DoS`-family budget `max_admitted_roots` (default
  `DEFAULT_MAX_ADMITTED_ROOTS = 32`) so a same-uid peer can no longer exhaust
  the daemon's descriptor table by admitting unbounded distinct roots.
  `AdmittedRoots` carries a `root_budget` (`with_root_budget`/`root_budget`/
  `root_budget_would_block`); `Confinement::to_admitted_roots_with_budget`
  and `SaveTimeState::with_root_budget` thread the resolved cap from
  `IpcLimits`; over-budget admissible roots are refused with a structured
  `SaveTimeError::RootBudgetExceeded` (`-32011`), distinct from `NotAdmitted`.
  Config merges stricter-wins (smaller cap wins). Budget enforced in both
  `Open` and `Allowlist` modes.

### CIB-155: Make Security Summary fail when its security scan jobs fail

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3103
- **Intent:** "Security Summary" is a required ruleset context, but its job
  runs `if: always()` and its `actions/github-script` step never calls
  `core.setFailed` based on the scan results — its conclusion is decoupled
  from whether `semgrep`/`dependency-audit`/`secret-scan`/`license-check`
  passed, failed, or were wrongly skipped. A red scan therefore cannot block
  a merge through the one security context branch protection actually
  requires. Surfaced by the CIB-137 verification review (PR #3098), which
  hardened the four scan jobs but could not reach this decoupling.
- **Expected Outcome:** the summary job fails (via `core.setFailed` or a
  preceding guard step) when any needed scan job resolves to `failure`, and
  treats the scans' `result` values alongside `needs.detect-changes` in the
  same `always()`/`result != 'success'` fail-closed pattern; scheduled/
  dispatch full-sweep behaviour unchanged.
- **Files:** `.github/workflows/security.yml` (summary job).
- **Validation:** a fixture/dry-run showing a forced scan-job failure turns
  "Security Summary" red (merge-blocking), while an all-green run and a
  schedule-triggered sweep stay green; existing PR-comment output unchanged.
- **Identified From:** CIB-137 verification review, 2026-07-03 (PR #3098).
- **Coordinates with:** CIB-137 (trusted classifier, merged), the security.yml
  fail-closed guards landed there.
- **Confidence:** high — a contained conditional/step change in one job, with
  the guard pattern already proven on the sibling jobs.

### CIB-156: Add fail-closed classifier guards to test-release-gate and build

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3102
- **Intent:** `ci.yml`'s `test-release-gate` and `build` jobs consume
  `needs.detect-changes.outputs.*` without the `always() &&
  (result != 'success' || ...)` guard the other consumers gained in CIB-137,
  so a classifier failure silently skips them. Neither is a required context
  today (release-gate only fires for `release/*`/`hotfix/*`-headed PRs;
  `build` excludes `pull_request`), so this is a narrower residual: an
  internal contributor on a release/hotfix PR could suppress the
  cross-platform Release Gate via classifier tampering or induced failure.
- **Expected Outcome:** both jobs adopt the uniform fail-closed pattern
  (fail-fast first step on `needs.detect-changes.result != 'success'`),
  keeping their existing trigger/branch conditions otherwise unchanged.
- **Files:** `.github/workflows/ci.yml` (`test-release-gate`, `build`).
- **Validation:** the structural assertions in
  `scripts/ci/fast-pr-validation.test.sh` extended to pin the guard on both
  jobs; YAML parse + fixture suite green; normal-path skipping behaviour
  demonstrably unchanged for a docs-only PR.
- **Identified From:** CIB-137 verification review completeness sweep,
  2026-07-03 (PR #3098).
- **Coordinates with:** CIB-137 (same guard pattern, merged); CIB-139
  (release-path tag trust, decision-gated).
- **Confidence:** high — mechanical replication of a pattern applied five
  times in CIB-137.

### CIB-157: Consolidate the three MCP path-safety implementations

- **Status:** In Progress — shared `collect_relative_files` exists in
  `shared.rs`; local `normalise_relative_path` copies remain in
  `apply_patch.rs` and `validate_write.rs`.
- **Intent:** `crates/anvil-cli/src/mcp/tools/` now has three independent
  "is this a safe workspace-relative path" checks: `shared.rs`'s
  `Component`-based `collect_relative_files`, `suppress.rs`'s
  `Component::Prefix`-based check, and CIB-148's byte-level
  `normalise_relative_path` in `query_boundary.rs`. `shared.rs`'s module doc
  explicitly exists "to stop per-tool copies from drifting apart", yet they
  now use different reject semantics. The CIB-148 review also confirmed
  empirically that on a Linux build target `Component::Prefix` never parses
  for Windows-style input (`C:\foo`, `\\server\share` become a single
  `Normal` component), so `suppress.rs`'s comment claiming those forms parse
  to `Component::Prefix` "on every platform" is inaccurate — the byte-level
  check in `query_boundary.rs` is actually the more correct model.
- **Expected Outcome:** a single shared, host-OS-independent path-safety
  helper (lexical normalise + reject `..`/absolute/drive/NUL) that all three
  MCP tools call; the misleading `Component::Prefix` cross-platform comment in
  `suppress.rs` corrected or removed. No behavioural change to the currently
  correct verdicts — this is a consolidation/accuracy fix.
- **Files:** `crates/anvil-cli/src/mcp/tools/{shared.rs,suppress.rs,fix.rs,query_boundary.rs}`.
- **Validation:** the shared helper carries the union of the current tests
  (drive/UNC/`..`/NUL/`./`/`//`/backslash); each tool's existing containment
  and boundary tests stay green; a test pins that Windows-style absolute forms
  are rejected on the Linux target (guarding against the `Component::Prefix`
  no-op).
- **Identified From:** CIB-148 adversarial review, 2026-07-03 (PR #3111).
- **Coordinates with:** CIB-145 (fix/suppress TOCTOU, in flight — shares these
  files; sequence after it merges to avoid churn), CIB-148 (merged).
- **Confidence:** medium — mechanically clear, but touches three live
  security-sensitive checks, so it needs careful test parity and should land
  after CIB-145 settles the same files.

### CIB-158: Close the anvil_fix/anvil_suppress write TOCTOU on Windows

- **Status:** Proposed — needs an owner decision on taking the `windows-sys`
  dependency/Hakari-churn risk now versus deferring until the next
  `windows-sys` bump lands, before the fd-path re-check can be wired.
- **Intent:** CIB-145 (PR #3113) closed the symlink-swap TOCTOU on Linux and
  macOS via a post-open fd→path re-check (`/proc/self/fd`, `fcntl F_GETPATH`)
  but deferred the Windows equivalent. On Windows the narrow
  canonicalise→open window stays open: unprivileged NTFS directory junctions
  (`mklink /J`, no `SeCreateSymbolicLinkPrivilege`) are followed transparently
  by `std::fs`, so an intermediate directory component can be swapped for a
  junction pointing outside the workspace between the containment check and
  the open. The wide read→write window is already closed on all platforms by
  handle-pinning; this is only the narrow open-time residual.
- **Expected Outcome:** `shared::handle_real_path` gains a
  `#[cfg(windows)]` arm using `GetFinalPathNameByHandleW` to re-derive the
  opened handle's real path and re-check workspace containment, mirroring the
  Linux/macOS arms; the per-platform residual table in `shared.rs` updates to
  show Windows intermediate/final-component swaps as blocked.
- **Files:** `crates/anvil-cli/src/mcp/tools/shared.rs`, and whatever
  `windows-sys` feature edge (`Win32_Storage_FileSystem`) the API needs
  (mind the [[windows-sys-hakari-churn]] pin discipline).
- **Validation:** a `#[cfg(windows)]` test proving an intermediate-component
  junction swap between check and write is rejected; the existing Unix tests
  stay green; `cargo hakari verify` clean after any windows-sys edge.
- **Identified From:** CIB-145 adversarial review, 2026-07-03 (PR #3113).
- **Coordinates with:** CIB-145 (merged), [[windows-sys-hakari-churn]]
  (dependency-pin landmine), CIB-157 (path-safety consolidation — same files).
- **Confidence:** medium — the API is a direct analogue of the shipped
  Linux/macOS arms, but it requires a `windows-sys` edge that has a history of
  Hakari churn, and it cannot be locally link-verified on this Linux box
  (needs a CI Windows runner).

### CIB-159: Scan a rename source path that still holds live bytes on disk

- **Status:** Draft
- **Intent:** CIB-151 (PR #3115) made `validate_paths` read and antipattern-scan
  the declared `desc.path` regardless of its wire change kind, but for a
  `Renamed { from }` descriptor only the destination (`desc.path`) is read. A
  `from` (source) path that still holds live bytes on disk — a copy rather than
  a true move — is never scanned unless the client separately sends a descriptor
  for `from`, leaving a residual path for unscanned live bytes to reach the
  tree.
- **Expected Outcome:** a `Renamed { from }` whose source path still resolves to
  readable content on disk has those source bytes read, hashed, and
  antipattern-scanned as well (or an explicit, documented decision that the wire
  protocol's `desc.path`-only contract makes this the client's responsibility).
- **Files:** `crates/anvil-intercept/src/validate_paths.rs`; possibly the
  `ChangeDescriptor`/`ChangeKindWire` wire types if `from` must be read too.
- **Validation:** a daemon test proving a `Renamed { from }` whose `from` path
  still holds live bytes on disk is read and antipattern-scanned; existing
  CIB-151 delete/rename destination coverage stays green.
- **Identified From:** CIB-151 security review, 2026-07-03 (PR #3115).
- **Coordinates with:** CIB-151 (merged), DSV-005/006 save-time verdict
  assembly, the antipattern check family.
- **Confidence:** medium — the read machinery already exists, but reading a
  second path per rename descriptor touches the wire contract (only `desc.path`
  is surfaced today) and needs an owner call on whether source scanning is the
  daemon's or the client's responsibility.

### CIB-160: Portable peer-exe check for durable-membership authorisation off Linux

- **Status:** Draft — needs an owner decision on whether a per-OS peer-exe
  reader is worth building, or whether the fail-closed non-Linux posture (never
  honour a wire durable claim) is the permanent answer given `--persist` /
  `register_on_start` already provide durability everywhere.
- **Intent:** CIB-150 (PR #3116) authorises a wire `agent_tag` durable
  claim by comparing the peer's `/proc/<pid>/exe` against the daemon's
  `current_exe`, which is Linux-only. On macOS and Windows there is no
  portable peer-exe reader wired, so `peer_authorised_for_durable_membership`
  returns `false` and every durable claim received over IPC is downgraded to a
  live/capped session. That is fail-closed and safe, but it means genuine
  `anvil start` / `anvil workspace register` durable membership over the wire is
  unavailable off Linux until the operator uses `--persist`
  (`register_on_start`, the in-process path that never crosses this
  dispatcher).
- **Expected Outcome:** `peer_authorised_for_durable_membership` grows a
  `#[cfg(target_os = "macos")]` arm (peer pid from `LOCAL_PEERPID` /
  `getsockopt`, exe via `proc_pidpath`) and a `#[cfg(windows)]` arm (peer pid
  from `GetNamedPipeClientProcessId`, image path via
  `QueryFullProcessImageNameW`), each comparing the resolved peer image against
  the daemon's own and each carrying an equivalent of the Linux
  faithful-foreign-read guard so a sandbox cannot force the gate open. The
  wire durable path then works on all three platforms without weakening the
  fail-closed default.
- **Files:** `crates/anvil-intercept/src/ipc.rs`
  (`peer_authorised_for_durable_membership`, `foreign_exe_reads_faithful`), plus
  whatever `libc` / `windows-sys` edges the per-OS readers need (mind the
  [[windows-sys-hakari-churn]] pin discipline).
- **Validation:** per-OS tests mirroring
  `dispatch_command_durable_claim_from_authorised_peer_persists` /
  `_from_non_anvil_peer_is_downgraded`, gated on the platform where the reader
  is enforced; the Linux behaviour stays unchanged.
- **Identified From:** CIB-150 adversarial review + CI run 28642478053,
  2026-07-03 (PR #3116) — the non-Linux gap was noted in the code's fail-closed
  doc comments but had no tracking item.
- **Coordinates with:** CIB-150 (merged), [[windows-sys-hakari-churn]]
  (dependency-pin landmine).
- **Confidence:** medium — the per-OS peer-pid/image APIs are well-trodden, but
  each needs a native runner to verify and the macOS/Windows faithful-read
  guard has no reference implementation yet.

### CIB-161: Reconcile the allowlist doc/CLI/diagnostic surface with fail-closed CIB-149

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3120
- **Intent:** CIB-149 (PR #3117) removed the implicit "primary check-in root"
  admission from `Allowlist` mode — the daemon now admits exactly the operator's
  configured allow entries (empty ⇒ nothing, fail-closed). The admission code is
  correct and stays as-is, but the operator-facing surface still describes the
  removed mechanism: the `anvil workspace` CLI text and clap help promise "plus
  each connection's primary root", the public config docs promise an empty
  allow-list "still serves each connection's primary check-in root so
  confinement never locks you out", the checked-in post-merge test plan describes
  the abandoned verified-primary design and cites non-existent tests, the refusal
  path logs no actionable diagnostic, and stale code doc comments still reference
  the implicit primary. No `[Unreleased]` CHANGELOG entry recorded the
  security-relevant behaviour change.
- **Expected Outcome:** every operator-facing description of `Allowlist` mode
  states the fail-closed contract (only configured allow entries are admitted; an
  empty list admits nothing; add roots with `anvil workspace allow <path>`); the
  refusal path emits a **server-side** warn carrying the refused `workspace_root`,
  the allow-entry count, and a remediation hint, while the wire reply stays
  static and path-free (N5 / CIB-091b); the post-merge test plan reflects the
  shipped fail-closed design and cites real tests; a `[Unreleased]` CHANGELOG
  Security entry records the change. No admission/confinement logic changes.
- **Files:** `crates/anvil-cli/src/commands/workspace.rs`;
  `docs/public/anvil/operations/config.md`; `CHANGELOG.md`;
  `plans/reviews/post-merge/fix-cib-149-verified-primary-root-allowlist.md`;
  `crates/anvil-intercept/src/{ipc.rs,save_time.rs,confinement.rs}` (doc comments
  + one server-side log line only).
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib`, clippy, and
  fmt stay green; the existing CIB-149 regression tests
  (`allowlist_empty_admits_nothing`,
  `allowlist_registered_session_worktree_is_not_admitted`,
  `registered_worktree_is_not_implicitly_admitted_in_allowlist`) are unchanged;
  `pnpm run format:check` clean on the edited `.md` files.
- **Identified From:** CIB-149 post-merge council review, 2026-07-04 (evidence:
  `plans/reviews/post-merge/cib-149-post-merge-council-review.md`).
- **Coordinates with:** CIB-149 (merged, PR #3117). The ADR-061 §7 amendment
  recording the fail-closed decision and the path-scoped `council-gate.sh` wiring
  are deferred as a separate governance PR needing owner sign-off (not in scope
  here).
- **Confidence:** high — mechanical doc/CLI-string/diagnostic corrections with a
  single behaviour-preserving server-side log-enrichment change; no admission
  logic touched.

### CIB-162: Human-render daemon-attestation skip warnings in `anvil start`

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3129
- **Summary:** Every `emit_skip_event` arm in `daemon_evidence.rs` demoted from
  `warn!` to `tracing::info!`, so the four operator-actionable skip reasons no
  longer surface as raw `{"timestamp":…,"level":"WARN"…}` JSONL mid-flow on the
  human `anvil start` / `--verify` surface (`info` sits below the default
  `warn` filter; JSONL still flows to `ANVIL_LOG=info` and file-sink
  consumers). Operator visibility stays owned by `render::daemon_evidence_label`,
  which folds every `DaemonAttestation` state into the human `daemon:` /
  `meaning:` lines. Tests pin no `{"timestamp"` line on stdout/stderr at the
  default filter (`tests/start.rs`, both surfaces), every skip reason at INFO,
  and non-empty human copy for every attestation state.

### CIB-163: Stop `anvil start` printing init's "Next: run `anvil start`"

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-03 via PR #3125
- **Intent:** When config is absent the orchestrator runs init inline
  (`orchestrator/mod.rs:170-173`) and init's success block ends with "Next:
  run `anvil start` to activate protection." (`init.rs:440-451`) — printed by
  `anvil start` itself, telling the user to run the command they just ran.
- **Expected Outcome:** Init invoked from the activation orchestrator prints a
  called-from-start variant (or suppresses its next-step line entirely, since
  the activation ending owns the next step); standalone `anvil init` keeps the
  current copy.
- **Files:** `crates/anvil-cli/src/commands/init.rs`,
  `crates/anvil-cli/src/activation/orchestrator/mod.rs`.
- **Validation:** `cargo test -p eddacraft-anvil init` plus a fresh-repo
  `anvil start` transcript with no "run `anvil start`" instruction in its own
  output.
- **Identified From:** User-journey pass 2026-07-04 (finding 2); reproduced
  live.
- **Confidence:** high.

### CIB-164: Make the `verify:` block honest about active layers

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3126
- **Summary:** The first-run `verify:` block no longer over-claims: the L3/L4
  commit+push hooks line prints only when `install_activation_hooks_silent`
  actually succeeded (per-hook results are now surfaced, not discarded), the
  wired-but-unattached L0 mcp pre-write layer is labelled pending rather than
  active, and the `.ts` smoke recipe plus "Next: run `anvil watch`" are
  suppressed when the diagnostic verdict is `unsupported`. Fixed in
  `start.rs`/`hooks.rs` with extended `first_run_recipe_*` fixtures
  (`crates/anvil-cli/src/activation/orchestrator/{install,mod}.rs`,
  `commands/start.rs`, `commands/hooks.rs`).

### CIB-165: Default the GitHub Actions workflow picker to unticked

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3179
- **Summary:** Both GitHub Actions picker options (PR validation, Nightly
  audit) now start unticked in `orchestrator/mod.rs`, so an Enter-through
  writes no `.github/workflows/*.yml`; ticking a workflow is the explicit
  consent. No extra confirm step or separate `anvil ci install` command.
- **Owner decision (2026-07-04):** default-unticked. Both workflow options
  start unselected; the user must actively tick to get CI files written, so a
  hurried Enter-through writes nothing. No extra confirm step and no separate
  `anvil ci install` command.
- **Intent:** Interactive `anvil start` shows "Install or enable GitHub
  Actions workflows?" with PR validation and Nightly audit both pre-ticked
  (`orchestrator/mod.rs:497-520`); Enter-through writes
  `.github/workflows/anvil.yml` + `anvil-audit.yml` — the most repo-visible,
  PR-triggering write activation performs, and the easiest to accept
  accidentally.
- **Expected Outcome:** Both picker options start unticked, so a hurried
  Enter cannot silently add CI workflows to a shared repo; ticking a
  workflow is the consent.
- **Files:** `crates/anvil-cli/src/activation/orchestrator/mod.rs`.
- **Validation:** `cargo test -p eddacraft-anvil orchestrator`; interactive
  transcript showing Enter-through writes nothing under the new default.
- **Identified From:** User-journey pass 2026-07-04 (finding 4); reproduced
  live.
- **Confidence:** high — posture decided 2026-07-04.

### CIB-166: One next-step arbiter per `anvil start` ending

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3137
- **Summary:** The diagnostic block's `next:` repair hint is the single
  arbiter: when it renders, it owns the ending and the closing `Next:` line
  is suppressed (`ending_next_step_line` gate in `commands/start.rs`,
  `has_repair_hint` exposed from `activation::render`); the closing line
  prints only at `Protecting`, where there is nothing to repair. Inline init
  already defers to the activation ending (CIB-163, #3125). An
  exactly-one-owner sweep pins the invariant across all six protection
  states, plus a regression test for the reproduced daemon-hint-vs-watch
  contradiction.
- **Intent:** A single first-run printed three competing instructions: init's
  "Next: run `anvil start`…" (`init.rs:440-451`), the diagnostic's "next:
  start the intercept daemon with `anvil intercept start --foreground`…"
  (`render.rs:757-761`), and the closing "Next: run `anvil watch`…"
  (`start.rs:851-859`) — the closing line can directly contradict the
  diagnostic's restart/daemon guidance. UJ-001's one-next-step-per-ending
  intent is defeated by three surfaces each owning a "next" line.
- **Expected Outcome:** One component owns the ending: the diagnostic `next:`
  and the closing `Next:` never disagree (either the closing line derives from
  the diagnostic's chosen next step or one of the two is dropped), and inline
  init defers to the activation ending (CIB-163).
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/render.rs`.
- **Validation:** `cargo test -p eddacraft-anvil start_next_step` extended to
  assert diagnostic-next and closing-next agreement across states.
- **Identified From:** User-journey pass 2026-07-04 (finding 5); reproduced
  live.
- **Coordinates with:** CIB-163 (init line), CIB-164 (verify block honesty).
- **Confidence:** medium — needs a small precedence design before mechanics.

### CIB-167: Improve activation state comprehension for terminal-first users

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3135
- **Summary:** Added additive plain-language `meaning:` lines for the
  `needs_action`, `unsupported`, and `watching` activation states and made the
  `needs_action`/`watching` copy honest (`render.rs`); the
  `restart_handshake_verified` / `server_startable` tier-token rename is
  deferred to an owner contract decision (spun out as a follow-up item) since
  the vocabulary is consumed byte-stably by `--verify` scripts.
- **Intent:** Terminal-first users park permanently on
  `ready_restart_required` with no editor to restart, and the MCP tier label
  `restart_handshake_verified` (`diagnostic.rs:87-96`) reads as success
  directly under a restart-required headline. Only `ready_restart_required`
  gets a `meaning:` line; `needs_action`, `unsupported`, and `watching` never
  do (`render.rs:576-605`).
- **Expected Outcome:** Additive `meaning:` lines for `needs_action`,
  `unsupported`, and `watching`; a decision note (owner call, since the tier
  vocabulary is a rendered contract consumed by `--verify` scripts) on
  renaming or glossing `restart_handshake_verified` / `server_startable` so
  tier tokens read as pending rather than done.
- **Files:** `crates/anvil-cli/src/activation/render.rs`,
  `crates/anvil-cli/src/activation/diagnostic.rs`.
- **Validation:** `cargo test -p eddacraft-anvil activation::render`; snapshot
  updates reviewed for byte-stability impact on `--verify` consumers.
- **Identified From:** User-journey pass 2026-07-04 (finding 6).
- **Confidence:** medium — additive copy is safe; the label rename needs the
  contract decision.

### CIB-168: Add a stop verb for the auto-started intercept daemon

- **Status:** Done — already shipped on main before pickup (verified
  2026-07-04)
- **Summary:** `anvil intercept stop` landed via PR #2781 (V060F-002) and was
  extended by PR #2958 (ACTMO-008/017): Unix sends SIGTERM so the daemon
  flushes fence state and exits, Windows stops the daemon and clears the PID
  file, and unsupported platforms bail with honest Ctrl+C guidance
  (`commands/intercept.rs`). The user-journey finding reproduced on the
  shipped 0.8.2-beta binary, which predates the ACTMO spine — no code change
  needed on main.
- **Intent:** `anvil start` auto-spawns the per-user daemon (ADR-082
  "activation is consent"), but `anvil intercept` offers only `start` /
  `status` / `unblock` (verified live on 0.8.2-beta and in
  `commands/intercept.rs` on main). The only off switches are prevention
  (`--no-daemon`) or `anvil uninstall --global`; a user who notices a new
  background process has no discoverable way to stop it.
- **Expected Outcome:** `anvil intercept stop` terminates the per-user daemon
  cleanly (socket/PID cleanup included), prints an honest line about what
  protection remains, and is named from the daemon lifecycle copy in
  `anvil start` output where relevant.
- **Files:** `crates/anvil-cli/src/commands/intercept.rs`,
  `crates/anvil-intercept/src/` (shutdown IPC or signal path).
- **Validation:** integration test: ensure → stop → probe shows no daemon;
  `anvil intercept stop` with no daemon running exits cleanly with honest
  copy.
- **Identified From:** User-journey pass 2026-07-04 (finding 7); verified live
  that no stop subcommand exists.
- **Confidence:** medium — needs a clean-shutdown IPC or signal design on all
  three platforms.

### CIB-169: Reconcile `anvil start`'s exit-0-on-auth-required with `&&` chaining

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3180
- **Summary:** Auth-required now propagates a distinct exit 3 on action
  commands (`anvil start` and siblings sharing the wall), superseding issue
  #1822's exit-0 mapping so `anvil start && deploy` no longer advances past
  an unactivated repo; read-only/status surfaces keep their contract. The
  `--help` exit-code table and CHANGELOG call out the beta breaking change;
  no `--strict` flag. Covered by `cargo test -p eddacraft-anvil auth`
  exit-code assertions.

### CIB-170: Make showcase findings unmistakably examples in discovery

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3127
- **Summary:** Showcase discovery findings are now unmistakably examples: an
  `is_showcase` flag on `ScanResults` (preserved through `filter_by_domain`,
  set at both `welcome.rs` showcase fallbacks) makes `render_findings_list`
  swap the panel title for an "Example findings — your scan found no issues"
  banner and prefix each row with a reversed `EXAMPLE` badge; real-scan renders
  are unchanged and the inline `[Example]` prefix is kept for copy robustness.

### CIB-171: Fix welcome TUI navigation scopes and init-summary honesty

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3131
- **Summary:** Three welcome-flow navigation/copy traps fixed. (a) `Esc` on the
  discovery results screen now backs out to the hub instead of advancing into
  the tutorial — exit classification extracted into a testable
  `discovery_outcome` helper (Back → back-out) and pinned with a unit test.
  (b) The init summary names the file actually written: the wizard always
  writes a single `.anvilrc` (format is the serialisation inside it), so the
  format-select labels no longer promise unwritten `.anvil.yaml/.json/.toml`
  files, a `CONFIG_FILE_NAME` constant is the single source of the name, and
  `generate_config` exposes the written path so the landing summary is derived,
  not hardcoded. (c) Hub sub-surface footers (gate/audit/doctor) read the
  honest "esc menu / q quit anvil" via an `embedded()` flag threaded into the
  surface states; standalone-command footer copy is unchanged. Snapshots
  updated.

### CIB-172: Windows variant for the first-run smoke recipe

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3145
- **Summary:** Branched the first-run smoke recipe's cleanup step in
  `start.rs` to render `del` on Windows and `rm` elsewhere (mirroring the
  tutorial's `cfg!(windows)` pattern) via named `RECIPE_CLEANUP_UNIX` /
  `RECIPE_CLEANUP_WINDOWS` consts and a `recipe_cleanup_line()` selector, so
  cmd.exe no longer hits `'rm' is not recognized`. Both variants are compiled
  and named on every host; a `first_run_recipe_cleanup_is_platform_branched`
  test pins the Windows `del` / no-`rm` contract.

### CIB-173: PATHEXT-aware editor detection on Windows

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3144
- **Summary:** Windows editor detection in `detect_agents.rs` now consults
  `PATHEXT` (bounded to `.exe`, `.cmd`, `.bat`, `.com`) so `.cmd`/`.bat` editor
  shims resolve and their MCP config is written, keeping the no-execute-bit
  false-match guard rationale documented; covered by unit tests over temp PATH
  dirs with `.cmd` shims.

### CIB-174: Align daemon bind-timeout copy with the real ceiling

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3146
- **Summary:** Ensure-failure recovery copy now names the effective wall-clock
  bound, deriving the printed figure from `(bind_timeout + PROBE_TIMEOUT)` in
  `crates/anvil-intercept/src/ensure.rs` (an in-flight probe can overrun
  `bind_timeout` by one `PROBE_TIMEOUT` by design), and the `start.rs` fixture
  literal was aligned to "12s". Pinned by a red-first `ensure_with` test through
  the spawn+never-answer path (PR #3146).

### CIB-175: Actionable watcher-failure guidance off Linux

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3147
- **Summary:** Watcher-start failures now render a platform-aware human line
  naming the likely cause and a next step, and partial-registration exhaustion
  is surfaced rather than silently dropping subtree coverage; Linux keeps its
  existing inotify-headroom preflight.

### CIB-176: Detect sh-less git before relying on `#!/bin/sh` hooks

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3149
- **Intent:** Activation-installed hooks are `#!/bin/sh` scripts
  (`hooks.rs:72-92`). Under standard Git for Windows this works (bundled
  MSYS sh), but a git lacking a bundled `sh` silently never executes them —
  the L3/L4 layer vanishes with no signal. The in-script
  `command -v anvil` degrade is good but never runs if `sh` itself is absent.
- **Expected Outcome:** Hook install (or `anvil doctor`) detects whether hooks
  can execute in the current git environment and reports honestly when they
  cannot; the `verify:` layer line reflects it (coordinates with CIB-164).
- **Files:** `crates/anvil-cli/src/commands/hooks.rs`,
  `crates/anvil-cli/src/commands/doctor.rs`.
- **Validation:** doctor check unit test with a simulated sh-less
  environment; existing hook tests stay green.
- **Identified From:** User-journey pass 2026-07-04 (finding 15).
- **Coordinates with:** CIB-164 (layer-claim honesty).
- **Confidence:** medium — detection heuristic needs care to avoid false
  alarms on healthy Git for Windows.

### CIB-177: Give bare `anvil` a first-run pointer instead of a 40-command dump

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3148
- **Summary:** Added a `before_help` banner on the root command (from new
  `help_layout::FIRST_RUN_POINTER`) pointing first-time users at `anvil welcome`
  (tour) and `anvil start` (activate); subcommand parsing and the exit-2/help
  contracts unchanged, covered by unit + bare-invocation integration tests.
- **Intent:** Plain `anvil` fails clap parsing (required subcommand,
  `main.rs:163-169`) and prints the full 40+-command help at exit 2;
  `welcome`/`start` are buried mid-list, so the very first contact for a new
  user is a wall of commands with no orientation.
- **Expected Outcome:** Bare `anvil` (or the help template) leads with a short
  orientation — e.g. an `about`/`before_help` line naming `anvil welcome`
  (tour) and `anvil start` (activate) — without changing subcommand parsing or
  exit-code contracts; CLIC-010 help-lint stays green.
- **Files:** `crates/anvil-cli/src/main.rs`,
  `crates/anvil-cli/src/help_layout.rs`.
- **Validation:** `cargo test -p eddacraft-anvil help`; `anvil` output names
  welcome/start above the command list.
- **Identified From:** User-journey pass 2026-07-04 (finding 16); reproduced
  live.
- **Confidence:** high.

### CIB-178: Exclude anvil-generated artefacts from the language profile

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3152
- **Summary:** Added an activation-only `is_anvil_owned_artifact(path, root)`
  predicate applied per-file in `profile_repo`
  (`activation/language_profile.rs`) so anvil's own writes (`.anvilrc`,
  `.anvil.<ext>` config, root-level `anvil/`, `.anvil-mcp-fallback.json`,
  installed `.github/workflows/{anvil,anvil-audit}.yml`) no longer inflate the
  language profile's unclassified count. Rules are root-anchored via
  `strip_prefix(root)` so nested `vendor/anvil/` and user `src/anvil.rs` are
  never dropped. TDD fixture asserts the unclassified count matches baseline and
  is stable across two runs. `plans/` deliberately left in scope to avoid
  dropping user planning docs.

### CIB-179: Say something when welcome surfaces drop copy in small terminals

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3150
- **Summary:** Both welcome renderers now reserve a trailing muted "resize for
  descriptions" hint in compact mode instead of silently dropping taglines and
  per-item descriptions, gated on genuine spare height so the fixed `Length(7)`
  logo is never traded for the hint row. Added a `compact_hint_never_squeezes_logo`
  invariant sweep (heights 8–32, `hint_shown ⇒ logo == 7 rows`) plus boundary and
  snapshot tests in `crates/anvil-tui/src/surfaces/welcome/render.rs` and
  `.../onboarding/welcome_render.rs`; no hard gate added (adaptive layout stays).
- **Validation:** `cargo test -p eddacraft-anvil-tui` compact-mode tests at
  40x12.
- **Identified From:** User-journey pass 2026-07-04 (finding 18).
- **Confidence:** high — additive rendering only.

### CIB-180: Decide whether MCP tier tokens should read as pending, not done

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3177
- **Summary:** Render-time gloss only: `render.rs` appends a "(pending
  restart)" qualifier beside tier tokens that read as done while a
  restart-required headline is active; machine tokens stay byte-stable
  (JSON/`--verify` asserted unchanged) and are documented as observed-probe
  state.
- **Owner decision (2026-07-04):** render-time gloss (option b). The
  machine tokens stay byte-stable; `render.rs` adds a human-facing pending
  qualifier next to the label (e.g. "(pending restart)") where the tier
  reads as done under a restart-required headline, and the tokens are
  documented as observed-probe state. No rename, no deprecation window.
- **Intent:** Under a restart-required headline, `restart_handshake_verified`
  reads as success ("verified") when the state is still pending a restart; a
  terminal-first user can misread the tier token as "done". The token names
  describe what was probed, not that protection has graduated.
- **Expected Outcome:** Human render shows a pending qualifier beside tier
  tokens that read as done while a restart is still required; JSON/`--verify`
  output stays byte-identical; the tier tokens are documented as
  observed-probe state (what was probed, not graduation).
- **Files:** `crates/anvil-cli/src/activation/diagnostic.rs`,
  `crates/anvil-cli/src/activation/render.rs`.
- **Validation:** render tests covering the pending qualifier per state;
  byte-stability assertion that `--json`/`--verify` output is unchanged.
- **Identified From:** CIB-167 (activation state comprehension); tier-vocabulary
  question deferred to the owner as a rendered-contract decision.
- **Confidence:** high — decided 2026-07-04 (render-only, non-breaking).

### CIB-182: First-user CLI command-path batch (local test 2026-07-07)

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12)
- **Summary:** All seven audit findings closed. Six via PR #3253
  ([#3216](https://github.com/eddacraft/anvil-001/issues/3216),
  [#3217](https://github.com/eddacraft/anvil-001/issues/3217),
  [#3218](https://github.com/eddacraft/anvil-001/issues/3218),
  [#3219](https://github.com/eddacraft/anvil-001/issues/3219),
  [#3221](https://github.com/eddacraft/anvil-001/issues/3221),
  [#3222](https://github.com/eddacraft/anvil-001/issues/3222));
  [#3220](https://github.com/eddacraft/anvil-001/issues/3220) (`ANVIL_HOME`
  wrong-mode cascade) via PR #3291 — owner-matched loose modes tightened to
  `0700` in socket/PID ensure paths, ensure pre-flights the runtime dir with a
  `chmod 700` recovery string, and quickstart/runbook document the requirement.
  Epic [#3223](https://github.com/eddacraft/anvil-001/issues/3223) closeout
  2026-07-31.
- **Intent:** Close the gap between ADR-082's interactive `anvil start` daily
  path and what first users actually see when running `welcome` → `start` →
  `status` → `watch` → `check` locally — especially false daemon-down signals,
  blocking pickers, and recovery copy that steers to `intercept start --foreground`.
- **Expected Outcome:** A new user following quickstart/local-dev guidance
  reaches `state: protecting` in one real-terminal `anvil start` without
  manual intercept launch; `anvil status` daemon line agrees with `anvil
  intercept status`; verify recipe and `anvil check` agree; non-interactive
  recovery copy leads with "run `anvil start` in a terminal".
- **Validation:** Re-run the audit repro in
  [`plans/audits/2026-07-07-local-cli-first-user-test.md`](../audits/2026-07-07-local-cli-first-user-test.md);
  each linked GitHub issue closes with an integration or transcript check;
  `cargo test -p eddacraft-anvil-intercept --lib tighten` and
  `ensure_dir_` filters green for #3220.
- **Identified From:** Local CLI command-path test session 2026-07-07 (this
  workspace, debug build).
- **Confidence:** high — reproduced on live binary before filing.
- **Coordinates with:** CIB-165 (workflow picker default-unticked — merged;
  discoverability gap remains), CIB-166/167 (activation copy — merged;
  non-interactive/intercept recovery drift remains), ADR-082 (daemon lifecycle).
- **Tracks (GitHub):**
  - [#3216](https://github.com/eddacraft/anvil-001/issues/3216) — status PID
    parse / daemon probe mismatch (**P0**, closed via #3253)
  - [#3217](https://github.com/eddacraft/anvil-001/issues/3217) —
    non-interactive recovery over-directs to intercept (**P1**, closed via #3253)
  - [#3218](https://github.com/eddacraft/anvil-001/issues/3218) — workflow
    picker blocks first `start` (**P1**, closed via #3253)
  - [#3221](https://github.com/eddacraft/anvil-001/issues/3221) — verify
    recipe / secret-detection gap (**P1**, closed via #3253)
  - [#3220](https://github.com/eddacraft/anvil-001/issues/3220) — `ANVIL_HOME`
    permissions cascade (**P2**, this PR)
  - [#3219](https://github.com/eddacraft/anvil-001/issues/3219) — watch warm-up
    completion signal (**P2**, closed via #3253)
  - [#3222](https://github.com/eddacraft/anvil-001/issues/3222) — status TUI
    blocks PTY/scripts (**P2**, closed via #3253)
- **Epic:** [#3223](https://github.com/eddacraft/anvil-001/issues/3223)

### CIB-183: Quiet repeat `anvil start` success output

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-11 via PR #3283
- **Summary:** A repeat `anvil start` whose run evidence shows the repo was
  already activated (config pre-existing, MCP entries up to date, no errors)
  now collapses to the protection state, daemon/driver posture, and one
  arbitrated next step; first-run and repair states keep the rich recipe, the
  TUI verdict reuses the same next-step arbiter, and a CIB-190 extra-line seam
  is reserved in the collapsed renderer.
- **Intent:** A repeat `anvil start` run should not reprint the full first-run
  recipe when the repo is already activated or the next action is already clear.
- **Expected Outcome:** Repeat-success activation output collapses to the
  protection state, daemon/driver posture, and one next step; first-run or repair
  states still render the richer recipe when it is useful. The compact plain
  path remains deterministic and the TUI path can reuse the same next-step
  arbiter without duplicating copy.
- **Validation:** `cargo test -p eddacraft-anvil start` with fixture or snapshot
  coverage for first run, repeat `protecting`, and repair-state output.
- **Identified From:** First-run council review C-008 in
  [`plans/reviews/2026-07-09-acttui-first-run-journeys.md`](../reviews/2026-07-09-acttui-first-run-journeys.md)
  and the earlier welcome/start user-journey audit.
- **Coordinates with:** CIB-166 (one next-step arbiter), ACTTUI-010 (plain/TUI
  contract fixtures).
- **Confidence:** high

### CIB-184: Default live MCP picker choices to unticked

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-11 via PR #3279
- **Summary:** The plain `anvil start` demand picker now offers every MCP
  candidate (`NotPresent` / `SafeDrift`) unticked, so an Enter-through writes
  no editor config; picker copy states the opt-in posture and non-interactive
  auto-install plus the UnsafeDrift refusal are unchanged.
- **Intent:** The live plain `anvil start` MCP picker should match the TUI
  consent posture: no editor config write is selected by default.
- **Expected Outcome:** In interactive plain mode, MCP candidates that are
  offerable (`NotPresent` / `SafeDrift`) start unticked, and pressing Enter with
  no explicit tick writes no MCP config. CI/piped/non-interactive auto-install
  policy remains unchanged where existing orchestrator rules allow it; unsafe
  drift remains refused. Help/copy makes the explicit opt-in posture clear.
- **Validation:** `cargo test -p eddacraft-anvil mcp_client start` covering
  demand-picker initial selection, Enter-without-tick no-write, and
  non-interactive policy parity.
- **Identified From:** First-run council review C-009 in
  [`plans/reviews/2026-07-09-acttui-first-run-journeys.md`](../reviews/2026-07-09-acttui-first-run-journeys.md).
- **Coordinates with:** CIB-165 (workflow picker default-unticked), ACTTUI-009
  (TUI consent wiring), ADR-044 (pinned the pre-selected picker; amended
  2026-07-11 to the unticked default by this item).
- **Confidence:** high

### CIB-181: Fix ETXTBSY flake in anvil-policy fixture-exec tests

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-05 via PR #3194
- **Summary:** `SubprocessRunner::eval_json`'s spawn site now retries on
  `std::io::ErrorKind::ExecutableFileBusy` (bounded, exponential back-off,
  `spawn_with_etxtbsy_retry`) — the ecosystem-standard fix for the
  inode-scoped multithreaded fork+exec race (mirrors golang/go#22315 and
  rust-lang/rust#114554). A write-to-temp-then-rename approach was
  considered and rejected: `rename` preserves the target inode's
  `i_writecount`, so it is a no-op for this specific race. Fixes all five
  fixture-exec tests routing through the `script()` helper; the item's
  original two `opa.rs`-based test references were dropped as dead (module
  deleted in ADR-098 PR-C). Validated with 20 consecutive
  `cargo test -p eddacraft-anvil-policy` runs, zero ETXTBSY.

### CIB-185: Make workflow consent writes race-safe against parent replacement

- **Status:** Proposed
- **Intent:** Keep an explicitly selected GitHub Actions workflow write bound to
  the parent directory that was consented, even if another process replaces or
  redirects `.github/workflows` between the consent probe and the write.
- **Expected Outcome:** Workflow installation uses a portable directory-bound
  or equivalent no-follow strategy plus atomic replacement, rejects parent
  symlink/reparse-point swaps at apply time, and cannot leave a truncated target
  if the write fails. The existing exact-selection and gated-write contracts
  remain unchanged.
- **Files:** `crates/anvil-cli/src/activation/orchestrator/mod.rs`, workflow
  installation helpers under `crates/anvil-cli/src/commands/` as discovered.
- **Validation:** Adversarial tests swap the workflow parent after consent and
  assert no write escapes the repository on Unix and Windows-supported paths;
  targeted activation tests and strict Clippy pass.
- **Identified From:** ACTTUI-009 milestone Council review; the race predates
  ACTTUI and was not widened by the consent wiring.
- **Coordinates with:** ACTTUI-009 (exact consent target binding), repository
  atomic-write utilities, Windows reparse-point handling.
- **Confidence:** medium — security boundary is clear; portable implementation
  needs a dedicated design pass.

### CIB-190: Healthy repeat-start local value receipt

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-12 via PR #3286
- **Shipped:** Collapsed healthy-repeat `anvil start` now carries one bounded,
  time-boxed (150 ms) local value line from the CIB-073 cumulative aggregate
  (risky writes flagged → saves checked → witness events, 30-day staleness
  horizon, omitted on any miss); promoted into the JOURNEY release cut per the
  operator-accepted conductor.
- **Intent:** Let a healthy repeat `anvil start` answer both "am I protected?"
  and, when reliable local evidence exists, "what has Anvil done?" without
  turning the confidence check back into onboarding noise.
- **Expected Outcome:** After CIB-183 collapses healthy repeat output, the result
  may include one bounded local value line sourced from trustworthy insights or
  witness aggregates (for example recent saves checked or findings caught).
  Missing, stale, ambiguous, or zero-filled evidence is omitted; the aggregate
  does not delay activation, leak repository details, or appear on repair paths
  where the recovery action must remain primary. TUI and compact plain paths use
  the same typed value, while JSON/verify contracts change only through an
  explicitly versioned owning schema.
- **Validation:** `cargo test -p eddacraft-anvil start`; `cargo test -p
  eddacraft-anvil insights`; fixtures cover healthy evidence, absent evidence,
  stale evidence, repair state, and redaction; repeat-start timing remains within
  the existing activation budget.
- **Identified From:** 2026-07-11 release user-journey review and accepted
  [`JOURNEY` conductor design](../specs/2026-07-11-release-user-journeys-conductor.md).
- **Coordinates with:** JOURNEY-003, CIB-183, CIB-073, INSIGHTS-001..005,
  ACTTUI-010.
- **Confidence:** medium — local evidence exists, but the owning aggregate must
  prove it is non-zero-filled and cheap before the line can render.

### CIB-191: Durable continuous-improvement pending queue and harvest

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-11 via PR #3285 — `scripts/ci-log/*`,
  `pnpm ci-log:*`, and `docs/guides/continuous-improvement-log.md` on main.
- **Intent:** Stop session CI-log notes from being lost when agents omit the
  tracked log from single-purpose feature PRs or when worktrees are removed
  before the note is committed.
- **Expected Outcome:** Default closeout writes to a shared pending queue under
  the git common dir (`.git/anvil/ci-log-pending/`), shared across Worktrunk
  worktrees and invisible to feature-branch `git status`. `pnpm ci-log:harvest`
  promotes pending notes into
  `plans/reviews/continuous-improvement-log.md`. All three `dev-workflow`
  adapters (Claude, OpenCode, Codex) require pending-first closeout.
  `pnpm test:ci-log` covers append/harvest/status/watermark. Operator guide at
  `docs/guides/continuous-improvement-log.md`.
- **Validation:** `pnpm test:ci-log`; `pnpm ci-log:status` shows pending path
  under git common dir; skills and project-context reference pending-first
  closeout; tracked log header documents harvest.
- **Identified From:** 2026-07-12 CI-log health review — ~122 commits / dozens of
  merged PRs after 2026-07-08 with almost no log growth; worktree scan found
  lagging logs rather than uncommitted newer entries (notes never landed).
- **Confidence:** high

### CIB-192: CI-log triage watermark and weekly disposition workflow

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-30 — tooling shipped with CIB-191 (PR #3285);
  Wave C triage completed (harvest, promote/absorb/leave, closeout note,
  watermark advanced). Weekly ops must continue: harvest pending, disposition
  entries since watermark, re-set watermark after each triage — do not treat
  this merge as "triage is done forever".
- **Intent:** Make the read→promote loop as real as the write path so pending
  and tracked evidence become CIB items (or deliberate leave/absorb decisions)
  instead of an unbounded append-only archive.
- **Expected Outcome:** Tracked log carries a `Last triaged` watermark;
  `pnpm ci-log:since -- --watermark` and `pnpm ci-log:set-watermark` support
  review; `.claude/workflows/triage-ci-log.js` runs status → harvest → review →
  promote/absorb/leave → watermark; Follow-up vocabulary
  (`none|session:|promote: CIB|theme:|owned:`) is documented in the log header
  and guide; session-start surfaces pending count.
- **Validation:** Watermark present in tracked log; `pnpm ci-log:since -- --watermark`
  exits 0; triage workflow file present; session-start prints CI-log status.
- **Identified From:** Same 2026-07-12 review — only one explicit promotion pass
  (2026-05-27) despite continuous write traffic when it was working.
- **Confidence:** high

### CIB-193: Finish the non-Linux cross-leg dead-code peel and close the PR-CI gap

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-26 via PR #3422 — `Clippy (windows-msvc)` runs on
  every Rust-touching PR. Its first runs found twelve pre-existing findings
  across eight crates, including a Windows test-compilation break from PR #3411
  and a same-day `needless_return` regression from PR #3419.
- **Intent:** PR #3290 fixed the cross-target cfg dead-code in anvil-intercept
  and anvil-bench, but dispatch run 29164882648 shows the next layer:
  `crates/anvil-cli/tests/start.rs` fails `-D warnings` on
  x86_64-pc-windows-msvc (`START_ACTIVATION_FIXTURES`,
  `start_activation_fixture_path`, `normalise_start_activation_output`,
  `assert_start_activation_fixture` are dead on non-unix). Peel it (and any
  crate beneath) the same way, and close the systemic gap the PR #3290 council
  flagged: no PR-triggered CI compiles these crates for non-unix targets under
  `-D warnings` (clippy is Linux-only; the cross matrix is
  release/hotfix/dispatch-gated; the nightly cross job is build-only).
- **Progress:** the dead-code peel half landed via PR #3297 (start.rs helpers
  gated `cfg(not(windows))`; full six-leg matrix green on dispatch run
  29168436934) — the remaining scope is the systemic PR-CI gap below.
- **Expected Outcome:** a cheap PR-triggered check (e.g.
  `cargo clippy --target x86_64-pc-windows-msvc --all-targets -p eddacraft-anvil-intercept -p anvil-bench -p eddacraft-anvil -- -D warnings`,
  or an equivalent check-only leg) so the cross-target cfg bug class fails
  PRs instead of nightlies.
- **Validation:** a PR that reintroduces a non-unix dead binding fails the
  new PR-triggered check; the check completes in minutes, not a full cross
  build.
- **Identified From:** PR #3290 council (session council-30437e3a,
  adversarial + operations findings); dispatch run 29164882648 on
  6d0fc2d712.
- **Confidence:** high

### CIB-194: Fix aarch64-apple-darwin base_store race-test failures

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-12 via PR #3297
- **Resolution:** a product-path platform race, not a test bug: macOS/APFS
  spuriously returns `ENOENT` under same-directory entry churn, hitting the
  create-once `.guard` `O_CREAT` open (fixed with a bounded yielding retry)
  and the `linkat` exclusive publish (fixed by reading the published record
  back and deciding by the full `{pid, start_time, nonce}` identity). Claim
  path errors now carry per-step tags for diagnosability. Both darwin legs
  green on dispatch run 29168436934.
- **Intent:** On dispatch run 29164882648 (HEAD 6d0fc2d712, files untouched by
  that PR), `snapshot_io::base_store::tests::concurrent_claim_is_single_flight_exactly_one_winner`
  and `reclaim_race_has_exactly_one_winner` panicked on aarch64-apple-darwin
  with `Result::unwrap()` on `Err(NotFound)` (base_store.rs:966/1128 in
  spawned threads), while the same tests passed on x86_64-apple-darwin —
  timing/arch-dependent, so either the test harness makes a filesystem
  assumption the arm runner breaks, or the single-flight claim/reclaim path
  has a real platform-sensitive race.
- **Expected Outcome:** Root cause identified (test-only vs product race);
  both tests deterministic and green on both darwin legs; if a product race,
  the fix reviewed as a GBASE trust-boundary change.
- **Validation:** Both named tests pass on aarch64-apple-darwin and
  x86_64-apple-darwin across a `rust.yml` dispatch; no `unwrap()` on
  filesystem results remains in the two test bodies.
- **Identified From:** PR #3290 council evidence run 29164882648
  (operations-reviewer finding C-001 residual).
- **Confidence:** medium

### CIB-195: Fix the TS OPA executor's real-binary path on Windows

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-26 via PR #3422 — resolved by **explicit
  retirement on Windows**, the second branch this item offered. See Results.
- **Intent:** The legacy TS policy executor's real-binary evaluations must
  either work on Windows or be explicitly retired there — never fail silently.
- **Expected Outcome:** On the Windows release-gate leg, every real-binary
  `opa eval` from `packages/anvil/policy` exits 2 with an EMPTY stderr —
  including the permitted-builtins control — so the whole real-binary path is
  broken there (capabilities semantics never enter into it). Diagnose the
  Windows spawn/eval failure (now made visible by surfacing stdout in
  `describeEvalFailure`), fix it, and remove the `process.platform === 'win32'`
  skip from the CIB-108 real-binary suite — or, if the JS/TS workspace
  retirement lands first, retire the suite with the workspace.
- **Validation:** Release-gate `Unit Tests (Node 22.x, windows-latest)` runs
  the CIB-108 real-binary describe block green with the skip removed.
- **Identified From:** v0.9.0-beta promotion PR #3301 — the release gate is
  the only CI context that runs this suite on Windows, and the suite
  (2026-07-02) postdates the previous release cut, so the gap was latent.
- **Coordinates with:** CIB-108 (the capabilities control — mock coverage
  still runs on Windows; real-binary coverage stays on unix legs), the JS/TS
  workspace retirement, CIB-132 (admin-key Windows nightly failures live in
  the same retiring workspace).
- **Confidence:** medium
- **Results:** Resolved by taking this item's second branch — **retire the
  real-binary leg on Windows explicitly**, rather than harden a path already
  scheduled for removal. The exclusion is now a named decision
  (`REAL_BINARY_RETIRED_ON_WINDOWS`) carrying its rationale, not a bare
  platform check that reads as owed work.

  The authority for this security property has moved. `crates/anvil-policy-engine`
  builds `regorus` without the `full-opa` bundle, dropping the `http` / `net` /
  `opa-runtime` builtin groups at **compile** time (that crate's
  `determinism.rs`): a policy calling `http.send` fails to resolve rather than
  being filtered at runtime. That is strictly stronger than this executor's
  `--capabilities` approach and identical on every platform, because nothing is
  spawned — so the Windows spawn failure has no analogue there. The sibling
  `opa-real.integration.test.ts` is already fully skipped pending the same
  regorus migration.

  What still runs on Windows: the mock-binary suite, which is what actually
  asserts this executor's contract (flag passed, denied built-ins filtered from
  the written profile, derivation failure fails closed). Only the leg that
  shells out to a real `opa` is excluded, and only there.

  **Not done:** the Windows `opa` spawn failure itself is undiagnosed. That is
  deliberate — diagnosing it needs a Windows host and would harden a retiring
  path. If the JS/TS workspace outlives expectations and the real-binary leg
  becomes load-bearing again, this needs reopening.

### CIB-196: prepare.sh changelog promotion needs manual curation

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-26 via PR #3422 — `prepare.sh` promotes the whole
  `## [Unreleased]` draft into a correctly-formatted section, refuses a cut with
  nothing to promote before bumping any version, and a rerun repairs a
  half-finished promotion.
- **Intent:** The release prepare step should promote the Unreleased draft
  into a changelog section that needs at most light review — not a structural
  rewrite during the cut.
- **Expected Outcome:** At the v0.9.0-beta cut, `prepare.sh`'s changelog
  promotion left a bottom metadata stub for the version in both changelogs,
  missed draft-worthy entries (the JOURNEY-wave items and the GBASE
  `ANVIL_PERSIST_GRAPH` default-on change), and retained an upgrade note that
  still described persistence as default-off. Curation took two follow-up
  commits on the promotion PR (`2315c3952`, `6b0ed1d1d`), including a
  Copilot-caught duplicated persistence bullet introduced during the
  hand-edit. Fix `prepare.sh` so the promoted section carries the complete
  Unreleased draft without stubs, and add the curation diff to the promotion
  PR review checklist so hand-edits get the same scrutiny as generated
  content.
- **Validation:** The next release's `prepare.sh` output passes promotion-PR
  review without structural changelog rewrites (no metadata stub, no
  missing-draft-entry curation); `scripts/release/_test` covers the promotion
  path.
- **Identified From:** v0.9.0-beta cut — curation commits
  `2315c3952`/`6b0ed1d1d` on promotion PR #3306; recorded in
  [`plans/releases/v0.9.0-beta.md`](../releases/v0.9.0-beta.md).
- **Coordinates with:** the `release` skill (`scripts/release/prepare.sh`),
  #3309 publication-recovery hardening (same release-mechanics intake wave).
- **Confidence:** high

### CIB-197: Stamp version and install method onto the command.invoked envelope

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-16 via PR #3351
- **Intent:** Every usage observation should be self-describing about the
  binary that produced it, so rows collected now stay analysable once any
  export or fleet surface exists.
- **Expected Outcome:** The `command.invoked` envelope
  (`anvil_intercept::kindling_observation::CommandInvokedObservation`) carries
  the producing binary's `version` and `install_method`. Version comes from
  the crate version the binary was built as; install method reuses the
  LAUNCH-013 `InstallMethod` detection already behind `anvil version`
  (`crates/anvil-cli/src/commands/version.rs`) rather than a second detector.
  Both fields are `serde(default)`-tolerant so existing daemon/sidecar rows
  and the TS-side validator keep parsing, and the privacy contract at
  `docs/observability/usage-analytics.md` documents them as low-risk
  dimensions (no path, no PII). The `anvil kindling usage` views keep working
  against mixed old/new rows.
- **Non-scope:** No data leaves the machine — this enriches the existing
  local Kindling pipe only. The consent posture, ingest endpoint, and any
  remote emission belong to the fleet-telemetry module (FLEET).
- **Validation:** `cargo test -p eddacraft-anvil` (full, unfiltered) plus
  `rg -n "install_method" crates/anvil-intercept/src/kindling_observation.rs docs/observability/usage-analytics.md`
  showing the field on the envelope and in the privacy contract.
- **Identified From:** 2026-07-14 operator observability review — fleet
  version/install-method visibility requested; local envelope enrichment
  split out as the no-consent-needed half.
- **Coordinates with:** [fleet-telemetry](./fleet-telemetry.aps.md) (FLEET —
  the remote half), USAGE privacy contract, KDS daemon store (mixed-schema
  rows in one SQLite store).
- **Confidence:** high

### CIB-198: Invisible-content trap antipattern rule (fragile-presentation family)

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-16 via PR #3357
- **Intent:** Flag content authored invisible (`opacity: 0`) whose visibility
  depends on an entrance animation firing, so a reduced-motion preference,
  backgrounded tab, or hydration miss can leave the section permanently blank.
- **Expected Outcome:** A new `patterns/fragile-presentation/` family
  (definition.anvil with the required six H2 sections) ships its first rule
  flagging the single-line motion entrance idiom
  `initial={{ opacity: 0, ... }}` in JS/TS/JSX/TSX source (RE2-safe pattern,
  no lookahead — pinned by the `registry_compile_diagnostics()` guard so a
  silently dropped rule fails tests, per the SPG precedent). Severity
  `warning`, confidence `medium`, on by default; allowlists cover
  tests/`__tests__`/`explain` fixtures. The remediation text routes to
  visible-by-default content (animate decoration, not existence; or animate
  from a visible initial state), noting suppression is appropriate where a
  visible no-JS/reduced-motion fallback provably exists — the ADR-087
  construction-smell posture: flag the sink, a human confirms the contract.
  The new category is registered end to end (`AntiPatternCategory` variant in
  `types.rs`, `map_category` arm in `registry_loader.rs`, TS
  `KNOWN_CATEGORIES` value) so it does not fall back to `code-quality`. Rule
  id prefix chosen at pickup (avoid `FP`, which reads as false-positive in
  dogfood reports; `FRAG-001` suggested).
- **Non-scope:** The other nine rules shortlisted from the same source.
  Render-time concerns (contrast ratios, clipped text, column alignment, dead
  controls, image seams) are out of scope for the static antipattern scanner
  entirely — they would need a render-time check category, which is its own
  ADR-sized decision. Multi-line reveal idioms (IntersectionObserver +
  classList opacity toggles) exceed the line-regex engine and are deferred
  until a JS/TS AST detection path exists.
- **Validation:** `cd packages/anvil/core && npx tsx
  scripts/compile-patterns.ts` then `oxfmt --write
  patterns/compiled/registry.json` leaves a clean committed registry;
  `cargo test -p eddacraft-anvil` (full, unfiltered) with fixtures showing the
  motion idiom fires the rule and a visible-by-default component does not;
  the compile-diagnostics guard reports no dropped `fragile-presentation`
  rules.
- **Identified From:** 2026-07-16 triage of the third-party pols.dev
  "anti-slop" design-law skill (~150 rules): seven correctness-grade
  candidates were assessed against the compiled registry (46 rules, ten
  families — no presentation coverage; a deliberately stacked probe file
  returned zero findings). This is the only shortlisted rule that is both
  statically detectable under the RE2 line-regex engine and a genuine
  correctness trap rather than a taste judgement.
- **Coordinates with:**
  [insecure-construction-catalogue](./insecure-construction-catalogue.aps.md)
  (INSEC — family authoring precedent),
  [anvil-scanner-parity-gaps](../archive/modules/anvil-scanner-parity-gaps.aps.md)
  (SPG — compile-diagnostics guard),
  [lang-ts-audit](../archive/modules/lang-ts-audit.aps.md) (LANGTS — AP idiom
  precedent).
- **Confidence:** medium — the regex idiom is clean and the family authoring
  mechanics are well-worn, but this is the first presentation-category family;
  category naming deserves a quick operator nod at pickup.

### CIB-199: Anti-pattern gate false-positives on committed generated files

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-22 via PR #3373, #3375 — Safe A generated-file
  exclusion Merged 2026-07-21 via PR #3373 (ADR-112 recorded in #3374), and the
  severity reconciliation Merged 2026-07-22 via PR #3375: `gate.rs` no longer
  hardcodes the `Warning` threshold and now derives it from the opt-in
  `--fail-on-warnings` / `ANVIL_FAIL_ON_WARNINGS`, restoring ADR-002
  warnings-over-blocks. The third disposition — generalising the finding-baseline
  to the anti-pattern family — was always scoped as its own ADR-gated module and
  is spun out as **CIB-207**; it does not hold this item open.
- **Intent:** A beta tester's `anvil gate` failed CI on `routeTree.gen.ts`
  (TanStack Router Vite plugin output). Generated files ship blanket
  `/* eslint-disable */` + `// @ts-nocheck` headers by construction, tripping
  `AP-001`; because `anvil gate` promotes `warning`-severity findings to
  blocking (exit 2), CI fails on code the user never wrote and cannot act on.
- **Findings (Council-confirmed, 5/5):**
  - **F1** — `anvil gate` promotes `warning`→blocking: `run_check_antipattern`
    (`crates/anvil-cli/src/commands/gate.rs`) hardcodes
    `severity_threshold: WarningSeverity::Warning`, overriding the engine
    default `Error` (`crates/anvil-checks/src/antipattern/types.rs`). It is the
    only override site; both MCP tool paths pass `Error`. This blocks on 27 of
    44 rules, including security-relevant `warning`-severity rules (weak crypto,
    unsafe rendering, JWT `none`). Contradicts ADR-002 (warnings-over-blocks;
    `fail-on-warnings` opt-in) and the "warnings over blocks" architecture
    principle.
  - **F2** — no generated-file exclusion in the anti-pattern scanner (AP-001
    allowlist covers only tests + `explain/`). `crates/anvil-checks/src/filter.rs`
    ships an unused `ScanFilter` with `*.gen.ts` capability.
  - **F3** — anti-patterns are never baselined (baseline/new-edges is
    SQL-drift-only), so the gate fires on pre-existing files, contrary to
    "new edges only".
- **Expected Outcome:** the three findings are dispositioned as follows —
  - **Safe A (PR #3373):** skip machine-generated files in the
    anti-pattern scanner core, detected by path convention (`*.gen.*`,
    `*.generated.*`, `generated`/`.generated`/`__generated__` segments) or a
    generator-attribution banner (`@generated`, `Code generated by`) — signal
    deliberately orthogonal to the `eslint-disable`/`@ts-nocheck` markers AP-001
    detects, so it cannot double as an author-driven gate bypass. Secret and
    other gate engines walk independently and are unaffected.
  - **Severity reconciliation (decided, see ADR-112):** promote the genuinely
    must-block security rules to `error`, drop the `Warning` override so
    warnings-over-blocks holds per ADR-002, and expose `fail-on-warnings` opt-in
    with a migration note. Separate follow-up PR; must not ship as a silent
    default flip. **Merged 2026-07-22 via PR #3375.**
  - **Baselining (spun out as CIB-207):** generalise the SQL-drift
    finding-baseline to the anti-pattern family as its own ADR-gated module —
    the durable "new edges only" answer, and the spoof-resistant alternative to
    content-trust. Design around grandfathering, regenerated-file fingerprint
    churn, and CI-runner snapshot persistence.
- **Non-scope:** Public reference docs (`rules.md`, `cli.md`) are
  generator-output; the `rules.md` "a warning does not automatically mean a
  command failed" line becomes true once the severity reconciliation lands, and
  an exclusion note belongs in the doc-generation source, not a hand-edit.
- **Validation:** `cargo test -p eddacraft-anvil-checks` (new
  `antipattern::generated` unit tests + `antipattern::check` scanner-integration
  tests, including a hand-written control proving the exclusion — not a broken
  trigger — suppresses the finding); `cargo clippy --workspace --all-targets`
  clean.
- **Identified From:** 2026-07-21 beta-tester report (TanStack Router
  `routeTree.gen.ts` gate failure) and the subsequent five-persona Council
  decision review.
- **Coordinates with:** ADR-112 (severity reconciliation), ADR-002
  (warnings-over-blocks).
- **Confidence:** high — F1/F2/F3 verified in-repo by all five reviewers;
  Safe A implemented in PR #3373 and green in CI; F1 discharged by PR #3375.

### CIB-200: Delegate package-manager-owned updates after explicit consent

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-24 via PR #3405
- **Intent:** Let `anvil update` complete an explicitly authorised update through
  the package manager that owns the running binary instead of stopping at a
  manual-command advisory.
- **Expected Outcome:** Homebrew, WinGet, and Scoop installs retain package-manager
  ownership and integrity verification while gaining a typed, allowlisted
  execution path. An interactive bare `anvil update` prints the exact command on
  stderr and asks `Run it now? [y/N]`; only an explicit yes launches it. `--yes`
  provides the same consent without a prompt, including for non-interactive
  callers. `--check` always remains read-only; `--version` and `--force` fail
  actionably on package-manager paths because the allowlisted latest-version
  commands cannot honour those requests. JSON mode never prompts, requires
  `--yes` before execution, and emits exactly one JSON document on stdout.
  Package-manager output is visible in human mode, a missing executable or
  non-zero exit is propagated, and declining leaves the installation unchanged.
- **Non-scope:** No arbitrary command execution, privilege elevation, manager-
  specific version selection, package-channel availability probe, new package
  manager, or change to the sidecar/library update and signature-verification
  paths.
- **Files:** `crates/anvil-cli/src/commands/update.rs`,
  `crates/anvil-cli/tests/update_resolution_chain.rs`,
  `README.md`, `docs/runbooks/cli-surface.md`, `plans/index.aps.md`, this module.
- **Validation:** `cargo fmt --all -- --check`;
  `cargo test -p eddacraft-anvil commands::update::tests`;
  `cargo test -p eddacraft-anvil --test update_resolution_chain`;
  `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`;
  `pnpm docs:check`; `pnpm aps:active-lint`; `pnpm aps:index:check`;
  `pnpm validate:changed`.
- **Identified From:** 2026-07-24 operator proposal after a Homebrew-owned
  `anvil update` stopped with only `brew upgrade eddacraft/tap/anvil` guidance.
- **Coordinates with:** ADR-025 package-manager distribution, ADR-045 update
  signing and delegated package-manager integrity, archived DISTRIB-001.
- **Confidence:** high — install-method detection and package-manager command
  mapping already exist; the remaining work is consent, typed process execution,
  output-mode discipline, and behavioural coverage.

### CIB-201: Refresh repository-local APS package from canonical vending

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-26 via PR #3407
- **Intent:** Replace the legacy root APS skill and stale harness agents with the
  current package emitted by the APS binary from canonical eddacraft assets.
- **Expected Outcome:** The repository carries current Claude, Copilot, and
  Codex APS agents plus managed `aps-planning` and `plan-doctor` skills under
  their native discovery roots. Legacy commands, the root `aps-planning/`
  tree, and the obsolete Codex configuration snippet are removed. All emitted
  Markdown is normalised by this repository's pinned oxfmt configuration.
- **Validation:** APS package update and doctor checks; `pnpm format:check`;
  `pnpm docs:check`; `pnpm aps:active-lint`; `pnpm aps:index:check`;
  `pnpm validate:changed`; `git diff --check`.
- **Identified From:** Canonical packaging refresh in eddacraft/skills#51 and
  eddacraft/anvil-plan-spec#131; follow-up to the inherited oxfmt failures in
  the previous copied APS/anvil skill update.
- **Confidence:** high
- **Results:** Migrated away from the root skill and slash commands, installed
  current APS agents for Claude, Copilot, Codex, and OpenCode, and installed
  managed `aps-planning` plus `plan-doctor` in the Claude and AGENTS.md-native
  roots. Added the required Git and formatter carve-outs for the new native
  paths so managed skill hashes remain fresh while `.github` agents stay under
  anvil's pinned formatter. APS doctor reports all four managed skill trees
  fresh. Format, docs, active APS lint, CIB index counts, and diff checks pass.
  Changed-scope validation passed except for nine daemon-absence assertions
  contaminated by a live local daemon; the isolated MCP tool slice passed all
  211 tests. Merged 2026-07-26 via
  [PR #3407](https://github.com/eddacraft/anvil-001/pull/3407).
  The pre-existing DASHCORE count mismatch remains an advisory.

### CIB-202: Flaky beacon reservation test under parallel load

- **Status:** Ready
- **Intent:** `telemetry::tests::reservation_commit_enforces_one_success_per_install_per_day`
  must be deterministic, or its non-determinism must be understood — a flaky
  assertion on the once-per-day beacon reservation erodes trust in the gate
  that protects users from repeated beacons.
- **Expected Outcome:** Root cause identified and removed, or the test made
  hermetic. Observed 2026-07-26 during a full `cargo test -p eddacraft-anvil`
  run: the third `reserve_beacon_in` returned something other than
  `Err(ReserveError::TooRecent)` at `crates/anvil-cli/src/telemetry.rs:1208`.
  Did not reproduce across three subsequent full runs, in isolation, or on
  `main`.
- **Notes for the diagnosis:** the usual suspects are already excluded. The
  test pins fixed timestamps (`2026-07-16T00:00:00Z` plus explicit offsets), so
  it is not wall-clock or day-boundary dependent, and `temp_dir()` is a unique
  `tempfile::tempdir()`, so it is not cross-test state collision. That points
  at either the commit not being durable before the next read, or
  `acquire_beacon_lock`'s non-blocking `try_lock_exclusive` returning `Busy`
  under load and the assertion mistaking one `Err` variant for another — the
  `matches!` asserts the specific variant but the failure message cannot show
  which variant it got.
- **Validation:** the test passes under repeated full-suite runs with the
  machine loaded; the assertion reports the actual variant on failure.
- **Identified From:** DASH-012 delivery-slice validation run, 2026-07-26.
- **Coordinates with:** USAGE-001 (the sidecar beacon surface).
- **Confidence:** medium

### CIB-203: Dependency Audit never runs on main, hiding latent advisories

- **Status:** Draft
- **Intent:** Make vulnerable dependencies on `main` visible when they land,
  rather than when an unrelated pull request happens to touch a lockfile.
- **Expected Outcome:** A HIGH or CRITICAL advisory affecting a dependency on
  `main` surfaces on a scheduled or push-triggered run, attributable to the
  commit or window that introduced it. Lockfile-touching pull requests stop
  inheriting unrelated red from advisories that predate them.
- **Validation:** Confirm `Dependency Audit` reports a non-skipped conclusion
  on a `main` run; confirm a pull request that changes no dependency is
  unaffected by a pre-existing advisory.
- **Identified From:** Review of PRs #3401, #3402 and #3404. All three failed
  `Dependency Audit` on packages absent from their diffs. The job in
  `.github/workflows/security.yml` is gated on
  `needs.detect-changes.outputs.dependency-audit-required == 'true'`, so on a
  plain `main` push it is **skipped**, and the workflow still reports success.
  Verified skipped on three consecutive `main` runs:

  | Commit      | Run         | Dependency Audit |
  | ----------- | ----------- | ---------------- |
  | `69a4c2823` | 30199921119 | skipped          |
  | `93b730d83` | 30194404983 | skipped          |
  | `b4b10cdc1` | 30170369907 | skipped          |

  Three HIGH advisories had accumulated unnoticed and only appeared because
  those PRs changed a lockfile: postcss (GHSA-r28c-9q8g-f849), quinn-proto
  (GHSA-4w2j-m93h-cj5j) and brace-expansion (GHSA-mh99-v99m-4gvg). Trivy scans
  the whole tree (`scan-ref: '.'`), so a PR touching any lockfile inherits every
  pre-existing finding — including `Cargo.lock` advisories for PRs that only
  change npm dependencies.
- **Confidence:** high
- **Notes:** The `dependency-audit-required` gate is a deliberate cost control
  (CIB-137 fails closed on classifier failure), so this is a scheduling
  question, not a request to drop the gate. Options worth weighing: a nightly
  or weekly unconditional audit on `main`; scoping the PR-triggered scan to the
  lockfiles actually changed so PRs are judged on their own diff; or both —
  they address different halves of the problem. Related remediation landed in
  [PR #3417](https://github.com/eddacraft/anvil-001/pull/3417).

### CIB-205: ACKNOWLEDGEMENTS prints one licence text per family, dropping the others' copyright notices

- **Status:** Ready
- **Intent:** Licence families whose members each carry their own copyright
  line must have every notice retained, not just one representative's. BSD
  3-Clause and MIT both condition redistribution on retaining "the above
  copyright notice".
- **Expected Outcome:** `ACKNOWLEDGEMENTS.md` carries each crate's own
  copyright notice for licence families that require notice retention, rather
  than a single representative text per family.
- **Evidence:** The `BSD 3-Clause "New" or "Revised" License` block has four
  members — `subtle`, `aws-lc-sys`, `regorus`, and now `matchit` — but prints
  exactly one licence text. Before `matchit` entered the graph the block
  carried `subtle`'s notice (Isis Agora Lovecruft, dalek-cryptography); after,
  it carries matchit's (Julien Schmidt) and `subtle`'s no longer appears
  anywhere in the file. The other two members' notices were already absent, so
  the gap predates this and is a property of the generator, not of any one
  dependency change. Surfaced by the DASH-012 regen
  ([PR #3421](https://github.com/eddacraft/anvil-001/pull/3421)), which added
  `matchit` and so changed which member is representative.
- **Validation:** for a multi-member family, every member's copyright notice is
  present in the generated file; adding a crate to an existing family does not
  remove another crate's notice.
- **Identified From:** DASH-012 ACKNOWLEDGEMENTS regeneration, 2026-07-26.
- **Coordinates with:** ATTRIB-006/-007 (the attribution pipeline and its
  freshness gate), `tools/starters/acknowledgements/`.
- **Notes:** legal-adjacent — this is an attribution-completeness question, so
  the fix (and whether the current output is already acceptable) wants an
  owner decision rather than an agent's judgement.
- **Confidence:** high

### CIB-204: Drain the Windows-only clippy backlog in anvil-intercept

- **Status:** Ready
- **Intent:** `anvil-intercept`'s Windows named-pipe transport must meet the
  same lint bar as the rest of the workspace, so the CIB-193 gate can cover it
  and the `--exclude` in `.github/workflows/rust.yml` can be deleted.
- **Expected Outcome:** every `cfg_attr(windows, allow(clippy::...))` marker
  in `crates/anvil-intercept/src/` naming this item is deleted, and
  `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc --
  -D warnings` still passes. Find them with
  `grep -rn 'cfg_attr(\s*windows' crates/anvil-intercept/src/`.
- **Where the baselines live:** deliberately split.
  `ensure.rs` (`unnested_or_patterns`, `manual_let_else`) and `interrupt.rs`
  (`collapsible_if`) carry per-site allows. The other five —
  `unused_self`, `large_enum_variant`, `unnecessary_wraps`, `too_many_lines`,
  `items_after_statements` — sit at the crate root in `lib.rs`, because their
  sites (`ipc.rs`, `registry.rs`, `save_time.rs`) are on the council-gate
  protected list: the same-uid save-time trust boundary. An inert `#[allow]`
  is not a trust-boundary change, and spending a Council review on three no-op
  attributes would train reviewers to wave through diffs on exactly the files
  that list exists to slow down. Clearing this item touches those files for
  real, so it *will* need the Council review — which is correct.
- **Findings to clear** (from the gate's first run, job 89803096820):
  - `ipc.rs:1442` — `too_many_lines` (114/100) on the named-pipe `serve`
    accept loop
  - `ipc.rs:1508` — `items_after_statements` (x3)
  - `ipc.rs:5022` — `unnecessary_wraps`
  - `registry.rs:328` — `large_enum_variant`
  - `save_time.rs:663` — `unused_self`
  - `interrupt.rs:463` — `collapsible_if`
  - `ensure.rs:783` — `manual_let_else`
  - `ensure.rs:713` — `unnested_or_patterns`
- **Why allows rather than an `--exclude`:** `cargo clippy` sets
  `RUSTC_WORKSPACE_WRAPPER`, which lints every workspace member built in the
  graph — so `--exclude eddacraft-anvil-intercept` does NOT suppress it while
  `anvil-cli` depends on it (verified: the findings still fired). Per-site
  allows are also the honest shape: each is a greppable debt marker that dies
  with the fix, rather than one flag hiding a whole crate.
- **Why this is its own item:** these are not the cfg-gated dead-code class
  CIB-193 closed; they are structural lints in daemon transport code. Splitting
  the accept loop, reshaping a hot-path enum, and unwrapping a `Result` in IPC
  cannot be compiled or tested anywhere but a Windows runner, so they want a
  reviewed change with Windows evidence — not a rider on the gate that found
  them. Landing them under CIB-193 would have meant refactoring daemon
  transport blind.
- **Validation:** the gate passes without `--exclude`; the six-leg cross matrix
  stays green; daemon IPC behaviour is unchanged on Windows (named-pipe
  connect, dispatch, shutdown).
- **Identified From:** first run of the CIB-193 `Clippy (windows-msvc)` gate,
  2026-07-26.
- **Coordinates with:** CIB-193 (the gate), ACTMO (Windows daemon-ensure
  chain), DSV-010/-011 (Windows save-time support).
- **Confidence:** high

### CIB-206: Worktree-anchor auto-heal generated stash debris and could reset staged work away

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-27 via PR #3440
- **Intent:** `scripts/dev/heal-primary-anchor.sh` runs as a `wt` post-switch /
  post-merge / post-remove hook on every worktree mutation. A hook that silently
  accumulates artefacts, or that can discard uncommitted work, undermines the
  anchor-strand protection it exists to provide.
- **Expected Outcome:** the heal serialises correctly across concurrent agents,
  never leaves a stash that captured nothing, and never resets away work that is
  not provably a strand.
- **Evidence:** 50 `primary-anchor: ... preserved for review` stashes had
  accumulated in the `anvil-001` clone; 27 captured nothing at all — working
  tree, index and untracked trees byte-identical to `HEAD`. Three defects:
  1. The `flock` file was `"${TMPDIR:-/tmp}/anvil-heal-primary-anchor.lock"`.
     Each agent process has its own `$TMPDIR`, so every agent took a private
     lock and nothing was excluded. Concurrent heals raced and a loser stashed a
     tree the winner had already reset. 21 stashes were created on 2026-07-27
     alone, 15 of them inside one minute (16:37:38–16:38:04).
  2. `git stash create` never includes untracked files, so an anchor holding
     only untracked files could never be proven a strand and was always stashed
     — hiding regenerable state such as `anvil/baseline.json`. `git reset
     --hard` does not touch untracked files, so they were never at risk.
  3. **Data loss:** `git stash create` commits the *working tree*, so staging an
     edit and then restoring the file yields a snapshot whose tree matches
     `HEAD` while the index does not — indistinguishable from a healed anchor.
     The strand proof matched it and `git reset --hard` destroyed the staged
     change outright, with no stash involved. Reproduced against the script as
     it stood on `main`. Surfaced by review on PR #3440, not by the original
     diagnosis.
- **Validation:** `scripts/dev/heal-primary-anchor.test.sh` covers 9 cases; the
  assertions added for each defect fail against the previous script and pass
  against the repaired one.
- **Identified From:** branch/stash cleanup sweep, 2026-07-27.
- **Coordinates with:** `docs/guides/worktree-policy.md` § "Default-branch
  anchor auto-heal", `.config/wt.toml` hook wiring.
- **Confidence:** high
- **Notes:** the 50 stashes were preserved as `refs/stash-backup/00..49` before
  clearing, and remain recoverable with `git stash apply refs/stash-backup/NN`
  until deliberately deleted. Defect 3 is the reason this is filed rather than
  left as a small-fix: the class of bug was live data loss, not just debris.
- **Post-merge confirmation (2026-07-30) — defect 3's fix held in the wild, and
  the residual hazard it leaves by design.** Observed in the primary `anvil-001`
  clone (the read-only anchor) during an APS reconciliation session. **No new
  defect**: the mechanism is the documented `wt` anchor stranding in
  `docs/guides/worktree-policy.md` § "Default-branch anchor auto-heal", and the
  heal behaved exactly as specified. Recorded because it is a real-world
  confirmation of this item's defect-3 repair plus a detection gap worth knowing.
  - **What happened.** The anchor was dirty — a genuine, long-lived staged APS
    package-vending change. `wt` kept fast-forwarding `refs/heads/main` to
    `origin` via its in-process git-library ref write, which moves the ref
    without updating the anchor's tree, so the anchor was left stranded behind
    `HEAD` and `git status` rendered the gap as the phantom "revert of merged
    work" the guide describes. The reflog signature matches that write: `main@{0..3}` had
    **empty** reason strings against `pull origin main: Fast-forward` on
    `main@{4}` and older, and `HEAD`'s own reflog never recorded those commits.
  - **Defect 3's fix is what protected the work.** Because the anchor's index
    disagreed with its working tree, the strand proof correctly failed and the
    heal refused to reset — precisely the behaviour added here ("staged changes
    are never treated as a strand"). Pre-repair, that state was
    indistinguishable from a healed anchor by tree alone and would have been
    hard-reset away. The vending change survived untouched.
  - **The residual hazard.** A dirty anchor therefore cannot be healed *by
    design*, and the strand deepens with every merge to `origin`, exactly as the
    guide predicts. Concretely: right after PR #3460 merged as `95c307794`,
    `plans/index.aps.md` showed staged as `ACTTUI | In Progress | 6/14` while
    `HEAD` held `Done | 14/14` — 15 lines on that file and 83 on
    `activation-tui.aps.md`, the exact inverse of the reconciliation that had
    just landed. Once `main` advanced again to `795816153`, twelve further files
    (`.github/workflows/*` ×10, `ACKNOWLEDGEMENTS.md`, `Cargo.lock`) took the
    same shape. A `git commit -a` in that anchor reverts merged work while
    looking like an ordinary "save my staged work" commit.
  - **Detection, when the anchor cannot be healed.** Per staged path, compare
    `git rev-parse :<path>` with `git rev-parse HEAD:<path>`; a staged blob
    matching an **ancestor** of `HEAD` is strand, not an edit. Repair with
    `git checkout HEAD -- <paths>` scoped to the confirmed-strand paths only,
    which preserves genuine work in the same anchor. Used here to clear four
    phantom-revert plan files after verifying each was byte-identical to the
    pre-merge commit, leaving the vending change staged.
  - **Possible follow-up (not filed):** nothing warns that a dirty anchor is
    accumulating strand. The heal exits 0 on this path by design, so the signal
    is only visible to someone who runs `git status` in the anchor and knows how
    to read it. A cheap advisory — heal reporting "anchor stranded N commits
    behind; N files will render as phantom reverts until the anchor is cleaned"
    — would close the gap. Filed nowhere yet; raise as its own CIB item if
    wanted.

### CIB-207: Generalise the finding-baseline to the anti-pattern family

- **Status:** Draft
- **Intent:** Anti-pattern findings are never baselined — the baseline and
  new-edges machinery is SQL-drift-only — so `anvil gate` fires on pre-existing
  files rather than on what the current change introduced. That contradicts the
  "new edges only" architecture principle, and it is the durable answer to the
  generated-file false-positive class that CIB-199 could only address by path
  convention.
- **Expected Outcome:** an ADR-gated design (and, on acceptance, its own module)
  for extending the finding-baseline to the anti-pattern family, so a gate
  warns on newly introduced anti-patterns and stays quiet on grandfathered
  ones. Baselining is also the spoof-resistant alternative to content-trust:
  unlike an in-file marker, a developer cannot opt their own new code out of the
  gate by writing a banner into it.
- **Design questions:** grandfathering semantics on first adoption;
  fingerprint churn when a generated file is regenerated; how a CI runner
  obtains and persists the baseline snapshot; interaction with the CIB-199
  generated-file exclusion once both exist.
- **Non-scope:** re-litigating the CIB-199 path-convention exclusion or the
  ADR-112 severity reconciliation — both shipped and stand on their own.
- **Validation:** ADR accepted and recorded in
  `plans/decisions/DECISION-LOG.md`; `pnpm docs:check`; `pnpm aps:active-lint`.
  Implementation validation belongs to the module this ADR spawns, not to the
  design item.
- **Identified From:** spun out of CIB-199's third disposition during the
  2026-07-30 reconciliation sweep; scoped there as its own ADR-gated module from
  the outset, so it was holding a fully-shipped item open.
- **Coordinates with:** CIB-199, ADR-112, ADR-002 (warnings-over-blocks), the
  existing SQL-drift baseline.
- **Confidence:** medium — the shape is clear and the precedent exists in the
  SQL-drift baseline, but grandfathering and CI snapshot persistence are
  genuinely open.

### CIB-208: APS lint missing work-item Status and taskless Ready modules

- **Status:** Draft
- **Intent:** APS validators currently accept modules marked Ready (and similar
  non-draft states) when individual work items omit required `Status` fields,
  and cannot flag modules that claim readiness without any work items. That
  lets stale or incomplete plans pass `aps:active-lint` while dashboard and
  release reconciles discover the gap by hand.
- **Expected Outcome:** `pnpm aps:active-lint` (or plan-doctor) fails or warns
  on (1) non-terminal work items missing a `Status` line, and (2) modules in
  Ready/In Progress with zero work items or only terminal items that cannot
  support the module status. Message names the file, item heading, and the
  missing field so operators can fix without grepping.
- **Validation:** Fixture modules covering missing Status, present Status,
  empty Ready module, and a healthy Ready module; active-lint green on the
  live corpus after any corpus fixes required by tightening the rule.
- **Identified From:** 2026-07-27 CI-log — "Reconcile current dashboard
  situation" (`promote: CIB`); APS validators accepted Ready modules whose
  work items omitted Status and could not detect impossible data-source claims.
- **Coordinates with:** CIB-036/037 (structural APS lint already Done), CIB-023
  (implemented-but-unreconciled — different drift class).
- **Confidence:** high — defect class observed in production reconcile; lint
  surface and validation path already exist.

### CIB-209: Worktree-safe local validation path selection

- **Status:** Ready
- **Intent:** `pnpm validate:changed` and agent-driven Rust/Nx checks repeatedly
  fail in Worktrunk worktrees for path-environment reasons that are not product
  defects: long Nx daemon sockets, read-only shared Cargo targets, `/tmp` as a
  nested Git worktree or full tmpfs, and `TMPDIR` paths long enough to break
  Unix sockets. Agents reinvent ad-hoc `CARGO_TARGET_DIR` / short-TMPDIR / store
  overrides every session.
- **Expected Outcome:** A repository-owned, documented validation entry (script
  and/or `validate:changed` behaviour) that selects writable, short cache and
  temp paths appropriate to Worktrunk + sandbox hosts; prefers disk-backed Cargo
  targets when tmpfs is constrained; reports inherited base (main) format/lint
  failures distinctly from task-local path failures; ignores task-local
  review/build debris that is not part of the change under test.
- **Validation:** In a Worktrunk worktree under a constrained sandbox, the
  wrapper completes a no-op or docs-only `validate:changed` without manual
  env surgery; fixture or documented dry-run proves short TMPDIR and non-tmpfs
  target selection when those conditions are simulated.
- **Files:** `scripts/validate/local.sh` (the `validate:staged` / `:changed` /
  `:full` entry), `.envrc` and `.config/wt.toml` (existing relocation
  convention this must read rather than duplicate),
  `docs/guides/worktree-policy.md`.
- **Identified From:** CI-log themes `theme:sandbox-safe-local-validation`
  (2026-07-24 PR #3379), `theme:worktree-validation-paths` (2026-07-27 WOW-006),
  `theme:rust-validation-storage` (2026-07-30 PR #3444) — three independent
  sessions, same class. Fourth occurrence 2026-08-04, and the first with a
  measured cost: ~27 GB of abandoned per-session `CARGO_TARGET_DIR` trees on the
  31 GB `/tmp` tmpfs (`mcp26-013-reverify-target`, `codex-mcp26-security-target`,
  `kfit006-verify.OgDbNF`, `verify-p7-target` and similar) took `/tmp` to 100%
  and broke unrelated tooling until cleared by hand. The same ad-hoc pattern is
  recorded inline in `plans/reviews/2026-06-27-cib-079-rust-ast-rules.md` and
  `plans/reviews/2026-06-26-cib-080-secret-fp-tuning.md`.
- **Coordinates with:** CIB-032 (stale global oxfmt on fresh worktrees),
  CIB-048 (shared Cargo target disk oversubscription — related but about
  capacity sharing, not path selection / sandbox writability). Sits directly on
  the residual porousness DEVENV-002 accepted when the committed
  `.cargo/config.toml` floor was dropped (cargo config `target-dir` does not
  expand `$HOME`), leaving relocation env-driven and therefore bypassable by an
  agent using neither direnv nor `wt`. `eddacraft/skills` PR #57 adds
  cache-location guidance to the `isolate-workspace` and `evidence-gate` skills,
  which narrows the agent-behaviour half but does not give the repository an
  owned entry point — this item is the durable fix.
- **Confidence:** high — recurring, multi-session, clear observable failures.

### CIB-210: Multi-worktree-safe PR merge cleanup

- **Status:** Draft
- **Intent:** `gh pr merge --delete-branch` can complete the remote merge and
  still exit non-zero when `main` (or the base branch) is already checked out in
  another local worktree, so agents report merge failure after a successful
  GitHub merge. Cleanup of the remote branch is blocked by the same local
  constraint.
- **Expected Outcome:** Dev-workflow / finishing-a-branch guidance (and any thin
  helper used at merge time) treats remote merge state as authoritative: if the
  PR is merged on GitHub, report success; delete the remote branch via API when
  local multi-worktree checkout constraints block `gh`'s local cleanup; never
  require checking out `main` in the task worktree to finish the loop.
- **Validation:** Documented procedure dry-run with base branch checked out in a
  second worktree; merge reports success when remote is merged; remote branch
  deletion succeeds without local base checkout.
- **Identified From:** CI-log `theme:multi-worktree merge cleanup` (2026-07-27
  DASHCORE-002) and CIB-200 closeout (2026-07-24) — same `gh pr merge` vs
  multi-worktree collision twice in three days.
- **Coordinates with:** CIB-028 (post-merge worktree cleanup sweep — removes
  worktrees after merge; this item is the merge command itself).
- **Confidence:** high — clear recurrence and an already-proven workaround
  (GitHub merge API path used in CIB-200).

### CIB-211: Authenticate supported Windows clients and trusted config ACLs

- **Status:** Ready
- **Intent:** Preserve the supported-surface security work split from CIB-114
  after retirement of the unsupported Node CLI framing.
- **Expected Outcome:** The TypeScript driver-client authenticates the named-pipe
  server identity before sending document traffic, and trusted Windows config
  reads reject files whose DACL grants write access to other principals.
- **Files:** `packages/anvil-driver-client/src/transport/windows.ts`,
  `crates/anvil-intercept-win32/src/lib.rs`.
- **Validation:** Windows tests reject a correctly named squatted pipe and an
  owner-matched but foreign-writable config file while preserving valid
  same-SID/SQOS connections and owner-only config reads.
- **Identified From:** supported-surface split from CIB-114 during PR #3464
  Council review (`council-891d78ba`).
- **Coordinates with:** CIB-100, CIB-106, MLP2-028 (peer-PID lineage).
- **Confidence:** medium — gaps and boundaries are concrete; Windows execution
  evidence remains the gating risk.

### CIB-212: Enforce real containment for exported APS loader paths

- **Status:** Ready
- **Intent:** Preserve the live APS-loader containment work split from CIB-115
  after retirement of the obsolete Node CLI framing.
- **Expected Outcome:** Module paths remain inside the plan base after symlink
  resolution; lexical prefixes, dot segments, alternate separators, and
  committed symlinks cannot escape the authorised plan directory.
- **Files:** `packages/aps/src/loader/index.ts`, focused loader tests.
- **Validation:** POSIX and Windows-path tests reject symlink and separator
  escapes while valid multi-module plans continue to load.
- **Identified From:** supported-surface split from CIB-115 during PR #3464
  Council review (`council-891d78ba`).
- **Confidence:** high — the exported loader and lexical-only containment path
  are directly testable.

### CIB-213: Contain runtime cache index paths before read or deletion

- **Status:** Ready
- **Intent:** Prevent a repository or restored cache index from steering the
  exported file-cache provider outside its cache directory.
- **Expected Outcome:** Every index filename is validated as a safe cache-local
  name and resolved containment is enforced before reads or `unlinkSync`;
  malformed entries fail closed without deleting unrelated files.
- **Files:** `packages/anvil/runtime/src/cache/providers/file-cache.ts`,
  focused file-cache tests.
- **Validation:** A malicious expired index entry such as
  `../../../../target` cannot read or delete outside the cache root; valid
  expiry cleanup and HMAC-protected entries remain stable.
- **Identified From:** supported-surface split from CIB-115 during PR #3464
  Council review (`council-891d78ba`).
- **Confidence:** high — the cleanup-to-invalidate path and containment
  invariant are local and deterministic.

### CIB-214: Redact structured debug payloads in the live API

- **Status:** Ready
- **Intent:** Preserve the live API redaction work split from CIB-116 after the
  unsupported Node admin CLI was archived.
- **Expected Outcome:** Structured debug values use the same recursive redaction
  boundary as scalar logs, so device codes, emails, tokens, and nested
  credential-shaped fields never reach console output.
- **Files:** `apps/anvil-api/src/lib/debug.ts`,
  `apps/anvil-api/src/routes/auth-device.ts`, focused logging tests.
- **Validation:** Captured console output proves nested device codes, emails, and
  token-shaped fields are redacted with `ANVIL_DEBUG` enabled; ordinary debug
  context remains useful.
- **Identified From:** supported-surface split from CIB-116 during PR #3464
  Council review (`council-891d78ba`).
- **Coordinates with:** CIB-121.
- **Confidence:** high — the raw structured logging path is directly reachable
  and testable.

### CIB-215: Remove credential material from provenance persistence

- **Status:** Ready
- **Intent:** Preserve the supported provenance work split from CIB-116 after
  the unsupported Node admin CLI was archived.
- **Expected Outcome:** Credential environment variables are never used as
  session identifiers, token-shaped agent ids are rejected or redacted before
  serialisation, and Git remote userinfo is stripped before provenance records
  or Git notes are persisted or published.
- **Files:** `packages/anvil/core/src/provenance/git-ai-standard/session.ts`,
  `packages/anvil/core/src/provenance/collector.ts`,
  `packages/anvil/core/src/provenance/git-notes.ts`,
  `packages/anvil/core/src/utils/git-operations.ts`.
- **Validation:** Regression tests prove Copilot-like tokens and
  credential-bearing HTTPS remotes never appear in stored records or Git notes;
  non-secret provenance remains stable.
- **Identified From:** supported-surface split from CIB-116 during PR #3464
  Council review (`council-891d78ba`).
- **Confidence:** high — the credential sources and persistence sinks are
  concrete.

### CIB-216: Route managed pre-push installs through the L4 runtime

- **Status:** In Progress 2026-07-31
- **Intent:** Managed file, Husky, and Git config pre-push installs currently
  invoke bare `anvil gate`, which scans the full worktree and blocks pushes on
  pre-existing findings instead of evaluating the pushed commit range. Route
  pre-push through the dedicated `anvil hook pre-push` runtime required by
  ADR-038 while preserving safe ownership and uninstall of legacy gate entries.
- **Expected Outcome:** Newly installed managed pre-push hooks consume Git's
  pre-push stdin through `anvil hook pre-push`, remain silent on a clean range,
  and no longer run a full-codebase quality gate. Existing managed
  `ANVIL_HOOK=1 anvil gate` config entries remain recognisable for status and
  uninstall during migration.
- **Files:** `.husky/pre-push`,
  `crates/anvil-cli/src/commands/hooks.rs`,
  `crates/anvil-cli/tests/hooks_config_mode.rs`,
  `crates/anvil-kernel-types/src/hooks.rs`.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types hooks`;
  `cargo test -p eddacraft-anvil --bin anvil hooks`;
  `cargo test -p eddacraft-anvil --test hooks_config_mode`; a synthetic
  pre-push ref update exits zero and produces no gate report.
- **Identified From:** 2026-07-31 repository dogfood pre-push failure: bare
  `anvil gate` scanned 1,667 files and blocked on 59 warning-level
  anti-patterns plus 540 secret-shaped fixture/cache matches, while
  `anvil hook pre-push` accepted the same pushed range.
- **Coordinates with:** ADR-038, GHOOK-002/GHOOK-004, MLP-004, MLP2-016, and
  CIB-207 (general finding-baseline design remains separate non-scope).
- **Confidence:** high — the dedicated runtime and accepted hook contract
  already exist; this corrects installer routing and migration detection.

### CIB-217: Distinguish ignored tool output from ignored credentials

- **Status:** Draft
- **Intent:** Full secret scans deliberately bypass blanket `.gitignore`
  filtering so ignored credential files such as `.env` remain visible, but the
  canonical local-noise policy does not recognise path-scoped generated output
  under tools such as `.clawpatch/**`, `.deepsec/data/**`, and `.vercel/**`.
  The result is hundreds of findings copied from old scans and fixture mirrors.
- **Expected Outcome:** Walking scan surfaces exclude declared generated
  tool-output paths without making `.gitignore` a secret-scan bypass, while
  tracked tool configuration and matcher source (for example `.deepsec/*.ts`)
  remains in scope.
- **Design questions:** whether path-aware exclusions belong in the
  kernel-canonical local-noise policy or an auditable project config surface;
  how a rule proves it cannot hide ignored credential files; how watch,
  audit, baseline, check, drift, and gate retain conformance.
- **Validation:** A fixture repo proves generated tool output is absent from
  secret and anti-pattern discovery, an ignored credential-shaped file remains
  detected, and tracked files adjacent to an excluded cache subtree remain
  scanned.
- **Identified From:** 2026-07-31 dogfood pre-push report plus
  `git check-ignore -v` confirmation for the reported cache paths.
- **Coordinates with:** ADOPT-004 (canonical local-noise list), CIB-207
  (grandfathering old tracked findings), and CIB-216 (pre-push range routing).
- **Confidence:** medium — the false-positive paths are concrete, but a blanket
  Git-ignore change would weaken secret protection and is explicitly rejected.

### CIB-218: Warn when a dirty anchor accumulates strand

- **Status:** Draft
- **Intent:** When the default-branch anchor is dirty, auto-heal correctly
  refuses to reset (CIB-206 defect 3) and exits 0. Nothing then reports that
  the anchor is stranded, so the gap widens silently with every merge to
  `origin` and the only signal is a `git status` a human must know how to read.
  The refusal path also parks a stash whose full label is
  `primary-anchor: unexpected changes in the '<default-branch>' anchor — NOT a
  provable wt strand; preserved for review`. That reads as preserved work
  even when every hunk in it is stale — an operator who trusts the label and
  applies it reverts merged work. The script already records the same hazard
  in a comment, describing regenerable state hidden behind "a 'preserved for
  review' label nobody read".
- **Expected Outcome:** The heal path emits an advisory when it declines to
  heal a dirty anchor, naming how far the anchor is stranded and which staged
  paths will render as phantom reverts, without changing its exit code or
  resetting anything. Any stash it parks is labelled so that "preserved" is not
  mistaken for "wanted".
- **Design questions:** whether the advisory belongs in
  `scripts/dev/heal-primary-anchor.sh` or a `doctor` surface that runs without
  a heal attempt; whether strand depth is reported per-path or as a single
  count; whether the parked-stash label should carry the classification verdict
  or only a pointer to how to obtain it; how the advisory stays quiet for a
  genuinely dirty anchor with no strand.
- **Validation:** A fixture anchor made dirty with a genuine staged edit, then
  advanced on `origin`, produces the advisory, still exits 0, and leaves the
  staged edit intact. A dirty anchor with no strand produces no advisory. The
  documented per-path test — `git rev-parse :<path>` versus
  `git rev-parse HEAD:<path>`, with a staged blob matching an **ancestor** of
  `HEAD` classified as strand rather than edit — is exercised directly.
- **Identified From:** 2026-07-31 session. `stash@{0}` (`f550d1a06`, parked
  2026-07-30, base `aaf108ecf` — not an ancestor of `main`, 39 commits behind)
  sat for two days labelled as preserved for review. Classification found every
  hunk stale: the `continuous-improvement-backlog.aps.md` blob was identical to
  `main` at `795816153` and would have deleted the live 45-line CIB-206
  post-merge note; the `aps-conductor` / `aps-librarian` / `aps-planner` agent
  files carried zero `mcp__anvil__*` entries against ten on `main`, reverting
  `b2e1bf2d3`; `aps-rules.md` removed the section preserving the
  `#cross-cutting-modules` link anchor and flipped `normalise` to `normalize`.
  Preserved at `refs/stash-backup/50` and dropped. Note that only the CIB file
  was catchable by a plain blob-versus-ancestor match — the other four had
  since taken unrelated commits, so a naive check reports them as genuine
  edits.
- **Coordinates with:** CIB-206 (the parent defect, where this was recorded as
  "Possible follow-up (not filed)"), and `docs/guides/worktree-policy.md`
  § "Default-branch anchor auto-heal".
- **Confidence:** medium — the gap and a second real occurrence are concrete,
  but the detection heuristic is known-incomplete, so scoping the advisory to
  strand depth rather than a per-path verdict may be the honest first cut.

### CIB-219: Reconcile internal and public documentation to current release truth

- **Status:** Released/Shipped via v0.9.2-beta (22f6a9be · 2026-08-04). Merged 2026-08-03 via PR #3500
- **Intent:** Complete the quarterly documentation audit across anvil's public
  user journey and its live internal entrypoints after the `v0.9.1-beta` cut.
- **Expected Outcome:** Public docs describe the shipped `v0.9.1-beta` command,
  telemetry, MCP, managed-skill, upgrade, and CI surfaces without future-tense
  `0.9.0-beta` framing; live repository entrypoints agree on the latest promoted
  release and the forward-looking release window; docs-site operator guidance
  matches the installed Docusaurus version, `pnpm` workflow, `docs/public/**`
  content roots, and static section configuration. Stale toggle guidance is
  removed instead of retained as executable procedure. Historical release,
  incident, and archive references remain historical.
- **Files:** `README.md`, `ROADMAP.md`, `RELEASE-PLAN.md`, `CHANGELOG.md`,
  `plans/index.aps.md`, `plans/project-context.md`,
  `plans/modules/continuous-improvement-backlog.aps.md`,
  `plans/modules/fleet-telemetry.aps.md`,
  `plans/modules/mcp-dual-era-support.aps.md`, `.github/workflows/ci.yml`,
  `eslint.config.mjs`,
  `apps/docs-site/AGENTS.md`, `apps/docs-site/README.md`,
  `apps/docs-site/TOGGLING-DOCS.md`, `apps/docs-site/sidebars/anvil.ts`,
  `docs/public/anvil/**`.
- **Validation:** `pnpm docs:check`; `pnpm docs:public:check`;
  `pnpm docs:public:commands`; `pnpm docs:public:aps-commands`;
  `pnpm release-plan:check`; `pnpm aps:active-lint`;
  `pnpm aps:index:check`; `pnpm --filter @eddacraft/docs-site build`;
  `pnpm validate:changed`.
- **Evidence:** PR
  [#3500](https://github.com/eddacraft/anvil-001/pull/3500) squash-merged as
  `f8f44308b40422683790315095874a355e6f5b84` after all required CI completed
  successfully. Council session `council-2ba5783e` converged with nine findings
  fixed, none open, and no waivers. Local docs, public-command, APS, formatting,
  workflow-contract, and docs-site build gates passed; inherited baselined link
  warnings and advisory aggregate APS counter drift remain outside this item.
- **Identified From:** 2026-08-03 quarterly documentation audit following the
  published `v0.9.1-beta` release and PR #3490's beta-entrypoint rewrite.
- **Coordinates with:** DOCSYNC (public anvil content), archived DOCGOV
  (authority/freshness contract), and DSITE (shared docs host wiring).
- **Non-scope:** Product behaviour, architecture decisions, sibling product
  editorial content, archive rewrites, and lifecycle reconciliation beyond
  documentation claims directly checked in this audit.
- **Confidence:** high — the published release, installed binary help, package
  manifests, docs-site config, and repository validators provide direct source
  truth for every correction.

### CIB-220: Project-scope interactive start must install MCP (not claim disabled)

- **Status:** Merged via [#3520](https://github.com/eddacraft/anvil-001/pull/3520) (`202b9c743`)
- **Priority:** P0 for `v0.9.3-beta` honesty pass
- **Intent:** `anvil start --mcp-scope project` in the interactive TUI currently
  routes through `legacy_mcp_install_policy`, which returns
  `McpInstallPolicy::Skip` for project scope and surfaces "MCP installation
  disabled". Scripted project-scope installs work. Users who need repo-local MCP
  hit a false dead-end in the TUI.
- **Expected Outcome:** Interactive project-scope start offers the MCP picker
  (or explicit client list) and installs via the scope-aware installer path;
  never claims MCP is disabled solely because scope is project. Plain/scripted
  and TUI paths agree. Regression tests cover TUI policy for project scope.
- **Files:** `crates/anvil-cli/src/commands/start.rs`
  (`legacy_mcp_install_policy`), `crates/anvil-cli/src/activation/orchestrator/`,
  `crates/anvil-tui/src/surfaces/activation/`
- **Validation:** `cargo test -p eddacraft-anvil start`; manual
- **Evidence (dev-loop):** `cargo test -p eddacraft-anvil --bin anvil -- tui_project_scope_orchestrator project_scope_disables` green; TUI uses `orchestrator_mcp_install_policy` → Install for project scope.
  `anvil start --mcp-scope project` in a clean worktree with at least one
  project-capable client.
- **Identified From:** Morgan Deus test of v0.9.1-beta (2026-08).
- **Coordinates with:** ACTTUI, ACTMO, MCPX, [#3510](https://github.com/eddacraft/anvil-001/issues/3510)
- **Confidence:** high — code and unit tests pin `Skip` for project scope today.

### CIB-221: Stop false auth-login prompts for already-authenticated pro users

- **Status:** Merged via [#3520](https://github.com/eddacraft/anvil-001/pull/3520) (`202b9c743`)
- **Priority:** P1 for `v0.9.3-beta` honesty pass
- **Intent:** Authenticated / pro users still see copy directing them to
  `anvil auth login` during start or related surfaces, which undermines trust
  after a successful login.
- **Expected Outcome:** When a valid session/token is present (and pro
  entitlement is satisfied where relevant), start/status/auth surfaces do not
  instruct the user to log in again. Unauthenticated users still get a clear
  login path. Cover with unit/integration fixtures for authed vs unauthed.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/`, `crates/anvil-cli/src/auth/`
- **Validation:** `cargo test -p eddacraft-anvil`; fixture with stored credentials
- **Evidence (dev-loop):** `unmarked_credentials_are_not_edicts` + `evaluate_auth_treats_unmarked_anvil_beta_token_as_ordinary_session` green; `is_edict` only when `Some(true)`.
  shows no login nag; without credentials, login guidance remains.
- **Identified From:** Morgan Deus test of v0.9.1-beta (2026-08).
- **Coordinates with:** AUTH, ACTMO, [#3510](https://github.com/eddacraft/anvil-001/issues/3510)
- **Confidence:** medium — needs repro against live auth state classes.

### CIB-222: Value receipt must disclose machine-wide vs repo-scoped evidence

- **Status:** Merged via [#3520](https://github.com/eddacraft/anvil-001/pull/3520) (`202b9c743`)
- **Priority:** P1 for `v0.9.3-beta` honesty pass
- **Intent:** Healthy repeat-start can show e.g. `value: N saves checked
  (dates)` drawn from machine-wide save-time aggregates (CIB-190). On a brand-new
  repo that never saw a save, the line reads as local activity. Insights
  scorecard already distinguishes "this machine" vs "this repository"; the start
  receipt does not.
- **Expected Outcome:** The one start value line names the evidence scope
  (machine-wide save-time vs repo witness) in plain language. No path or repo
  names leak. Zero/stale/missing evidence still omits the line.
- **Files:** `crates/anvil-cli/src/commands/start.rs` (`repeat_value_line`),
  optionally `crates/anvil-cli/src/insights/`
- **Validation:** `cargo test -p eddacraft-anvil start` value-line cases;
- **Evidence (dev-loop):** `value_receipt_*` suite green; save-time says "on this machine", witness says "for this repository".
  fixture with machine-wide saves and empty repo witness includes scope wording.
- **Identified From:** Morgan Deus test of v0.9.1-beta (2026-08).
- **Coordinates with:** CIB-190, INSIGHTS, JOURNEY, [#3510](https://github.com/eddacraft/anvil-001/issues/3510)
- **Confidence:** high — copy gap on an existing aggregate.

### CIB-223: Coherent non-git init vs worktree registration messaging

- **Status:** Merged 2026-08-04 via [#3522](https://github.com/eddacraft/anvil-001/pull/3522) (`ce362650a`)
- **Priority:** P2 for `v0.9.3-beta` honesty pass
- **Intent:** A non-Git directory can init successfully, then immediately hear
  there is no worktree and registration cannot proceed — jarring sequential
  truths without a single next step.
- **Expected Outcome:** Either (a) refuse durable init outside a git worktree
  with a clear error, or (b) allow init but one message: config written; git
  init/register before protection can attach. No success-then-contradiction.
- **Product choice:** (b) soft path — allow durable init/config writes outside
  git; one coherent next-step message (not success-then-contradiction).
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/registration.rs`, activation orchestrator init step
- **Validation:** integration/unit for non-git cwd; message contract tests.
- **Evidence (dev-loop):** soft-path worktree line names config-may-be-written +
  `git init` / `anvil workspace register` before protection can attach; unit
  message contract covers non-git cwd.
- **Identified From:** Morgan Deus test of v0.9.1-beta (2026-08).
- **Coordinates with:** ACTMO-016, LAUNCH, [#3510](https://github.com/eddacraft/anvil-001/issues/3510)
- **Confidence:** medium — product choice required (refuse vs soft path).

### CIB-224: Reject --no-mcp with explicit MCP client selection

- **Status:** Merged 2026-08-04 via [#3522](https://github.com/eddacraft/anvil-001/pull/3522) (`ce362650a`)
- **Priority:** P2 for `v0.9.3-beta` honesty pass
- **Intent:** `anvil start --no-mcp --mcp-client codex` (and similar) silently
  ignores the client instead of erroring. Operators cannot tell whether install
  was skipped by design.
- **Expected Outcome:** Mutually exclusive flag set fails with exit non-zero and
  a one-line recovery: drop `--no-mcp` or drop client selection. Same for
  `--all-mcp-clients` / `ANVIL_ALL_MCP_CLIENTS` with `--no-mcp` / `ANVIL_NO_MCP`.
- **Files:** `crates/anvil-cli/src/commands/start.rs` (early validation next to
  other mutual-exclusion bails)
- **Validation:** `cargo test -p eddacraft-anvil start` conflict cases.
- **Evidence (dev-loop):** early bail next to watch mutual-exclusion; unit tests
  for `--no-mcp`+`--mcp-client`, `--no-mcp`+`--all-mcp-clients`, and env forms.
- **Identified From:** Morgan Deus test of v0.9.1-beta (2026-08).
- **Coordinates with:** ACTMO, ADR-092, [#3510](https://github.com/eddacraft/anvil-001/issues/3510)
- **Confidence:** high — pure validation gap.

### CIB-225: Warn when --format is ignored because config already exists

- **Status:** Merged 2026-08-04 via [#3522](https://github.com/eddacraft/anvil-001/pull/3522) (`ce362650a`)
- **Priority:** P3 for `v0.9.3-beta` honesty pass
- **Intent:** `--format toml` (etc.) on a tree that already has `.anvilrc` /
  `.anvil.*` is a silent no-op; users think format changed.
- **Expected Outcome:** When `--format` is set and a project config already
  exists, emit one stderr warning that the flag was ignored and name the
  existing path; do not rewrite or convert formats unless an explicit migrate
  command exists later.
- **Files:** `crates/anvil-cli/src/commands/start.rs` (`pre_write_anvil_config`
  call site)
- **Validation:** unit test with existing `.anvilrc` + `--format toml`.
- **Evidence (dev-loop):** pre-write returns existing path for skip; stderr
  warning names path; unit test `.anvilrc` + `--format toml`.
- **Identified From:** Morgan Deus test of v0.9.1-beta (2026-08).
- **Coordinates with:** MLP2-039, [#3510](https://github.com/eddacraft/anvil-001/issues/3510)
- **Confidence:** high.

### CIB-226: Public CLI docs — current flags and auth exit code 3

- **Status:** Merged 2026-08-04 via [#3525](https://github.com/eddacraft/anvil-001/pull/3525) (`36c01a243`)
- **Priority:** P3 for `v0.9.3-beta` honesty pass
- **Intent:** Public CLI reference / support docs omit current `anvil start`
  flags and the auth-required exit code **3** on action commands (status stays
  0). Beta testers cannot map failures to help text.
- **Expected Outcome:** Public docs list current start/MCP flags and document
  exit codes including action-command auth exit 3 vs read-only status exit 0,
  aligned with `crates/anvil-cli/src/main.rs` and live `--help`.
- **Files:** `docs/public/anvil/reference/cli.md`,
  `docs/public/anvil/reference/support.md`, related quickstart links
- **Validation:** `pnpm docs:public:check`; `pnpm docs:public:commands` if
  applicable; spot-check against `anvil start --help`.
- **Identified From:** Morgan Deus test of v0.9.1-beta (2026-08).
- **Coordinates with:** DOCSYNC, CIB-219, [#3510](https://github.com/eddacraft/anvil-001/issues/3510)
- **Confidence:** high.
- **Evidence (in progress):** Extended
  `scripts/docs/generate-anvil-public-reference.mjs` so generated `cli.md`
  lists current `anvil start` flags (from `commands/start.rs`) and the stable
  exit-code map (auth action exit **3**, read-only `status` exit **0**).
  Public troubleshooting + upgrade notes link the same contract. Validation:
  `node scripts/docs/generate-anvil-public-reference.mjs --check` (pass);
  `node scripts/docs/check-public-docs.mjs` (0 errors / 68 files).

### CIB-227: User-facing copy must not imply only Claude Code and Cursor

- **Status:** Merged 2026-08-04 via [#3525](https://github.com/eddacraft/anvil-001/pull/3525) (`36c01a243`)
- **Priority:** P2 for `v0.9.3-beta` honesty pass
- **Intent:** Product still has twelve-client install (MCPX) but several live
  surfaces still describe MCP wiring as Claude Code + Cursor only (or those two
  as the documented set). That undercuts multi-client beta truth after 0.9.x.
- **Expected Outcome:** Public docs, bundled skills, and activation as-built
  user-facing claims use multi-client wording (examples + "see
  `anvil mcp install --help`") unless a sentence is truly about a
  Claude/Cursor-only capability (e.g. legacy HTTP preview). Inventory and fix
  the exclusive-pair list under `docs/public/`, `crates/anvil-cli/assets/skills/`,
  and activation runbooks that beta testers hit.
- **Files:** `docs/public/anvil/reference/support.md`,
  `crates/anvil-cli/assets/skills/anvil-developer-functions/**`,
  `crates/anvil-cli/assets/skills/using-anvil/SKILL.md`,
  `docs/runbooks/anvil-no-mcp-activation.md`,
  `docs/architecture/activation-as-built.md` (user-claim sentences)
- **Validation:** `rg` for exclusive Claude/Cursor pairs on public/skill paths
  returns only intentional legacy-HTTP exceptions; `pnpm docs:check`.
- **Identified From:** operator follow-up after multi-client ship; related to
  Morgan multi-client pass on Deus.
- **Coordinates with:** MCPX, DOCSYNC, CIB-219, [#3510](https://github.com/eddacraft/anvil-001/issues/3510)
- **Confidence:** high — inventory already enumerated.
- **Evidence (in progress):** Support generator reads the full
  `AgentClientId` install registry (12 clients); skills, no-MCP runbook, and
  activation as-built user claims reword exclusive Claude/Cursor pairs.
  Historical v1 tables note launch-time scope. Exclusive-pair `rg` on scoped
  paths clean; public-docs check 0 errors.

### CIB-228: Fix PowerShell dual-install guard inject (silent no-op)

- **Status:** Merged 2026-08-04 via PR #3526
- **Priority:** P0 for `v0.9.3-beta` (Windows install path)
- **Intent:** The official Windows `irm … | iex` installer is a silent no-op on
  clean machines because the package-manager dual-install guard was written as a
  standalone pre-check (`exit 0` = continue) and is prepended verbatim to
  cargo-dist's `eddacraft-anvil-installer.ps1` at release publish time.
- **Expected Outcome:** Clean PATH (no WinGet/Scoop `anvil`) runs the full
  cargo-dist install body (download, checksum, place, receipt). Dual-install
  still refuses with upgrade guidance and non-zero exit. No second top-level
  `param` block before cargo-dist's params. Contract tests cover (a) fall-through
  on clean PATH and (b) refuse-on-dual-install. Still present on published
  `v0.9.2-beta` installer — must ship in the next Windows-facing cut.
- **Files:** `scripts/install/windows-package-manager-guard.ps1`,
  `.github/workflows/release.yml` (global-artifacts inject),
  `scripts/install/_test/windows-package-manager-guard.test.sh` (and any inject
  assembly fixture)
- **Validation:** `pnpm test:windows-pm-guard` (or successor); fixture that
  concatenates/injects the way release.yml does and asserts the cargo-dist body
  remains reachable; manual `irm` against a pre-release asset on a clean Windows
  VM when available.
- **Identified From:** Dave beta feedback on v0.9.1 (2026-08); re-verified on
  published v0.9.2-beta installer asset.
- **Coordinates with:** closed dual-install intent [#2885](https://github.com/eddacraft/anvil-001/issues/2885)
  (regression of the inject shape), CIB-230, [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high — live public asset still has happy-path `exit 0` at the
  top of the injected guard.

### CIB-229: Align cargo-dist receipt layout with update + install-method detection

- **Status:** Merged 2026-08-04 via PR #3526
- **Priority:** P0 for `v0.9.3-beta` (self-update + version honesty)
- **Intent:** Three symptoms share one root cause family: (1) `anvil update
  --check` fails with axoupdater "The updater isn't properly configured";
  (2) `anvil version` labels a cargo-dist install under `CARGO_HOME/bin` as
  "cargo install" and suggests `cargo install --git --force` on hosts with no
  Rust; (3) receipt path / app name in code do not match what cargo-dist
  installers write (`eddacraft-anvil-receipt.json` under the
  `eddacraft-anvil` config dir). Absorbs former CIB-231 (UPD-3).
- **Root cause (do not fix half):**
  - `AxoUpdater::new_for("anvil")` while installers write app name
    `eddacraft-anvil`.
  - `has_cargo_dist_receipt` looks for `…/anvil/anvil-receipt.json`.
  - `classify_exe_path` returns `CargoInstall` for any `CARGO_HOME/bin` path
    **before** consulting a receipt — cargo-dist's default layout is that
    path, so classification can never win even with a correct receipt.
  - No-receipt library fallback sets version + source but not
    `install_prefix`, so `is_update_needed*` still NotConfigured.
- **Expected Outcome:**
  - After a cargo-dist shell/PowerShell install: `anvil update --check`
    reports current vs available (or up-to-date) without bare NotConfigured.
  - Same install: `anvil version` / install-method is cargo-dist (not cargo
    install); upgrade guidance is installer / `anvil update`, not
    `cargo install --git`.
  - True `cargo install` builds remain classified correctly.
  - Receipt load tries `eddacraft-anvil` (and legacy `anvil` if needed).
  - No-receipt / dev builds: actionable message or fully configured check
    path — never bare NotConfigured as the only signal.
  - Package-manager paths (Homebrew/WinGet/Scoop) unchanged.
- **Non-scope:** Restoring `install-updater` sidecar while
  `aarch64-pc-windows-msvc` axoupdater is missing; Authenticode.
- **Files:** `crates/anvil-cli/src/commands/update.rs`,
  `crates/anvil-cli/src/commands/version.rs` (`classify_exe_path`,
  `has_cargo_dist_receipt`, upgrade command strings),
  `crates/anvil-cli/tests/update_resolution_chain.rs`, version unit tests
- **Validation:** unit tests for receipt name + classify order (CARGO_HOME +
  eddacraft-anvil receipt → CargoDist; CARGO_HOME without receipt stays
  CargoInstall when that is true); `cargo test -p eddacraft-anvil
  commands::update::tests`; `commands::version` classification tests;
  optional manual `--check` on a receipt-backed install.
- **Identified From:** Dave field report UPD-1 + UPD-3 (2026-08); re-triage
  2026-08-04 merged UPD-3 into this item.
- **Supersedes:** CIB-231
- **Coordinates with:** ADR-045, CIB-200, CIB-228, dist-workspace
  `install-updater = false`, [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high — all three defects visible in source.

### CIB-230: No internal GH issue numbers in public ship artefacts

- **Status:** Merged 2026-08-04 via PR #3526
- **Priority:** P2 for `v0.9.3-beta` (release hygiene; land with CIB-228)
- **Intent:** Private tracker ids (e.g. `GH #2885`) were baked into the public
  PowerShell installer banner and help text. Anyone who downloads the asset sees
  the internal number even though the private issue stays 404 unauthenticated.
- **Expected Outcome:** Public installers, release notes that ship to
  `eddacraft/anvil`, binary user-facing strings, and published changelogs use
  neutral wording (e.g. "package-manager dual-install guard") without
  `GH #NNNN` unless the tracker is intentionally public. A lightweight check
  (script or test needle) fails CI/release when a known private-id pattern
  appears in release-injected installer text. Internal plans/APS may still
  reference issue numbers.
- **Files:** `scripts/install/windows-package-manager-guard.ps1`,
  `.github/workflows/release.yml` inject banners, any release-note generators
  that copy APS issue refs into public assets
- **Validation:** `rg` / contract test on the inject pipeline output forbids
  `GH #` / `GitHub #` in the published installer body; spot-check next pre-release
  asset.
- **Identified From:** Dave beta feedback research note (2026-08) — they cited
  #2885 because it was in the public installer, not because of private repo
  access.
- **Coordinates with:** CIB-228, [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high — live on v0.9.1-beta and v0.9.2-beta assets.

### CIB-231: Report cargo-dist installs as cargo-dist (not cargo install)

- **Status:** Done 2026-08-04 — superseded by CIB-229 (same root-cause family:
  receipt path + `classify_exe_path` order; re-triage after Dave field review).
- **Summary:** UPD-3 folded into CIB-229 so agents fix receipt name,
  classification order, and update --check together rather than half-fixing
  either surface.
- **Superseded by:** CIB-229
- **Identified From:** Dave field report UPD-3.

### CIB-232: Disclose open admission mode honestly (do not flip factory default)

- **Status:** Merged 2026-08-04 via PR #3529
- **Priority:** P3 presentation (Dave CONF-1; re-triaged 2026-08-04)
- **Scope narrowing (recorded 2026-08-04):** delivered on `anvil workspace list`
  and `anvil workspace mode open` only. `anvil status` is deliberately left
  alone: its save-time line appends `· confined: N` **only** in allowlist mode
  (`status.rs` `confinement_allow_count`), so in open mode it makes no
  confinement claim at all — silence, not a misleading one. Adding posture copy
  to the flagship one-line status block is a wider product-copy decision than
  this P3 presentation item; file a follow-up if operators want it there.
- **Intent:** Fresh home reports `Admission mode: open` with no entries. Open
  is the **intentional** factory posture (`Open` = first-touch adopt; missing
  config → `open_default()`; fail-closed is only for untrusted config). Dave
  read open + empty allow as "decorative ceremony" against an enforcement
  pitch — the defect is **undisclosed posture**, not the default itself.
- **Non-scope / do not:** Silently change factory default to allowlist-empty
  (that bricks intercept until every root is registered). A default flip
  requires an explicit product ADR and is out of this item.
- **Expected Outcome:** `workspace list` / related surfaces state in one plain
  line that open mode means confinement is off (first-touch adopt) until the
  operator sets allowlist — without implying a bug. Docs already call open the
  default; keep them aligned. Optional later ADR if product wants closed-by-
  default.
- **Files:** `crates/anvil-cli/src/commands/workspace.rs` list/status copy;
  any onboarding string that names admission mode
- **Validation:** fresh home `anvil workspace list --no-tui` includes honesty
  that open = not confined; default mode remains open in tests.
- **Identified From:** Dave CONF-1; operator re-triage: intentional default.
- **Coordinates with:** confinement `open_default`, [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high on product stance; fix is copy-only.

### CIB-233: audit-chain summary must disclose coverage (do not redefine chain_intact)

- **Status:** Merged 2026-08-04 via PR #3534
- **Priority:** P2 presentation (Dave TRUST-1; re-triaged 2026-08-04)
- **Intent:** With all commits unwitnessed under default `--threshold 5`,
  `chain_intact: true` and `degraded_audit_drift: false` are **correct** under
  the current contract: `chain_intact` = witness file DAG not tampered;
  `degraded_audit_drift` = unwitnessed count ≥ threshold. Dave read the multi-
  field report as a green health pass. Fix presentation / summary, not field
  semantics.
- **Non-scope / do not:** Redefine `chain_intact` to mean "every commit was
  witnessed" (breaks existing tests and kindling observation consumers). Do
  not change default threshold solely to make N=2 fail without a separate
  product decision.
- **Expected Outcome:** Human (and JSON summary if present) lead with
  **coverage** (e.g. witnessed 0/N, unwitnessed list or count) so zero-coverage
  cannot be skimmed as "all good". Keep `chain_intact` / `degraded_audit_drift`
  meanings stable and documented in help or report schema notes. Optional:
  document that `--no-tui` is JSON for this command if that stays intentional.
- **Files:** `crates/anvil-cli/src/commands/audit_chain.rs` human print path /
  summary fields
- **Verified during implementation:** `chain_is_intact` returns `true` when
  the witness directory is **absent** as well as when the DAG verifies, and a
  truncated tail leaves a verifying prefix — so the summary must not describe
  it as tamper-evidence. The rendered qualifier says only that the records
  present verify and that missing records are not detectable there; coverage
  is what catches the missing-records case.
- **JSON:** no new field. `witnessed` and `commits_walked` are already
  first-class and already precede `chain_intact` in the serialised order, so
  the "JSON summary if present" clause is satisfied without touching the
  byte-exact `anvil.audit-chain.v1` empty-state pin.
- **Validation:** fixture 0/2 witnessed, threshold 5: output clearly states
  unwitnessed coverage; `chain_intact` may still be true; regression tests for
  field semantics unchanged.
- **Identified From:** Dave TRUST-1; re-triage against audit-chain module docs.
- **Coordinates with:** CIB-234..236, ADR-037 audit-chain,
  [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high on presentation gap; contract must stay.

### CIB-234: `audit` must disclose its secret domain vs `check` (not force count parity)

- **Status:** Merged 2026-08-04 via PR #3534
- **Priority:** P2 presentation (Dave TRUST-2; re-triaged 2026-08-04)
- **Intent:** Same planted-secret tree: `check --all` showed 4 secret findings;
  `audit` showed 2 (file-level `.env` + summary). `audit` and `check` are
  **different product surfaces** (overview vs rule-engine findings); identical
  counts are not a requirement. Silent under-count that looks like a cleaner
  bill of health is the problem.
- **Non-scope / do not:** Force audit to emit one issue per check finding
  without a product decision; do not change gate/check secret rules solely to
  match audit aggregation.
- **Expected Outcome:** Prefer (b): audit states its domain (e.g. security
  overview, file-level `.env`, summarised source secrets) so a lower count
  cannot be read as "check is wrong" or "tree is cleaner". If a real secret
  missed entirely by audit's stated domain, fix that as a separate finding.
  Optional later: product ADR for full parity.
- **Files:** `crates/anvil-cli/src/commands/audit.rs` messaging / aggregation
  summary
- **Scope narrowing (recorded, not silent):** the disclosure ships on the
  plain and JSON output paths only, matching this item's `Files` field. The
  default `anvil audit` surface on a TTY is the **TUI** (`OutputMode::Tui`),
  and the SARIF path feeds GitHub code scanning; neither carries the scope
  note, because both live in `crates/anvil-tui/` (18 `AuditData` construction
  sites across two crates). Follow-up item owed for those surfaces — until
  then the Expected Outcome is met for plain/JSON, not for TUI/SARIF.
- **Verified during implementation:** audit and `anvil gate` select the same
  secret **file types** by documented invariant (`SECRET_SCAN_EXTS` lock-step
  with gate's `matches!` arm, issue #1798) — but *not* the same file set:
  gate's full-codebase walk caps at `SECRET_SCAN_MAX_DEPTH` (20) and narrows
  to plan files under `anvil gate <plan>`, neither of which audit does. The
  copy therefore says "types", not "set". Planless `anvil check` runs only
  `PLANLESS_ELIGIBLE_CHECKS` (`secret-detection`, `antipattern-scan`), so the
  disclosure routes readers to `anvil gate` as the full suite and frames the
  difference as *which checks run*. A `.env` yields one file-level flag
  **plus** one entry per pattern match.
- **Follow-up found, not fixed here (out of scope):** the depth cap means a
  `.env` nested deeper than 20 levels is flagged by `audit` and missed by
  gate's `secret-detection` — audit flagging a file gate ignores, the exact
  direction issue #1798's lock-step comment warns against. Needs its own item;
  changing gate's traversal is out of scope for a presentation fix.
- **Validation:** planted multi-file secret fixture; audit output names domain;
  no requirement that issue count equals `check --all`.
- **Identified From:** Dave TRUST-2; re-triage.
- **Coordinates with:** CIB-233, CIB-255 (gate/`check --all` domain — same
  disclosure stance, different surfaces),
  [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high on presentation; medium if residual true misses found.

### CIB-235: `status` Protection:warming must name next step or refuse that label

- **Status:** Merged 2026-08-04 via PR #3544
- **Priority:** P2 (Dave TRUST-3)
- **Intent:** `status` can show `Protection: warming` in a state that will never
  warm, with no named next step, while leaking internals
  (`subordinate:`, `ready_restart_required`). `start --verify` meaning lines are
  already actionable — status should match that bar.
- **Expected Outcome:** Warming/degraded postures either include a concrete next
  command or use a posture label that cannot be read as "wait and it will
  finish". Internals stay out of default human output (verbose/json ok).
- **Files:** `crates/anvil-cli/src/activation/posture.rs`,
  `crates/anvil-cli/src/commands/status.rs`
- **Validation:** fixture never-attach context; human status names the restart
  or setup action and does not expose internal posture labels.
- **Identified From:** Dave field report 2026-08-04 TRUST-3.
- **Coordinates with:** CIB-220 honesty, start --verify meaning lines,
  [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Pack-02 extension (STATUS-2, 2026-08-04):** A never-activated repo can
  report `L3 commit partial` / `L4 push partial` while `doctor` correctly says
  hooks not installed / unprotected. Same honesty family as warming — layers
  must not read as partially on when nothing is installed. Prefer one fix pass
  with TRUST-3 rather than a second CIB.
- **Confidence:** high.

### CIB-236: `insights` zeros must disclose the counted domain

- **Status:** Merged 2026-08-04 via PR #3544
- **Priority:** P3 (Dave TRUST-4)
- **Intent:** All-zero insights counters do not say what domain was counted
  (zeros may be true for unattested contexts; rendering gap).
- **Expected Outcome:** Zero and non-zero insights lines name the evidence
  domain (e.g. this machine / this repo / unattested) in plain language,
  consistent with CIB-222 value-receipt scope wording where applicable.
- **Files:** `crates/anvil-cli/src/commands/insights.rs`
- **Validation:** zero and non-zero weekly-summary fixtures name repository or
  machine evidence domains; JSON/schema output remains unchanged.
- **Identified From:** Dave field report 2026-08-04 TRUST-4.
- **Coordinates with:** CIB-222, [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high.

### CIB-237: Consistent path and line rendering across CLI surfaces

- **Status:** In Progress
- **Priority:** P3 (Dave UX-1)
- **Intent:** Three path styles across surfaces — relative (`src/app.py`),
  NT-extended (`\\?\C:\...`), unix-ish (`/.env:1`) — plus `.env:0` zero-based
  line in audit and mixed separators in skill-install output.
- **Expected Outcome:** User-facing paths prefer one style per platform
  (relative when in-repo, normalised absolute otherwise); line numbers are
  1-based in all human/json locations; skill-install paths use consistent
  separators.
- **Files:** check/audit/gate location formatters, skill install messages
- **Validation:** Windows fixture covering check --all, check <file>, pre-commit
  gate, audit, skill install.
- **Identified From:** Dave field report 2026-08-04 UX-1.
- **Coordinates with:** [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Pack-02 absorb (PATH-1):** `status --json` emits mixed separators in one
  value (e.g. `.git\\hooks/pre-commit`). Covered here; no separate CIB.
- **Confidence:** high.

### CIB-238: Clarify "Blocking warnings" means threshold-block, not severity=warning

- **Status:** Ready
- **Priority:** P3 polish (Dave UX-2; re-triaged 2026-08-04)
- **Intent:** Banner text is already "Blocking warnings found **(severity meets
  threshold)**" — "warnings" means findings that trip the block threshold, not
  `severity == warning`. Dave read it as mislabeling errors. Logic is fine;
  vocabulary is overloaded.
- **Non-scope / do not:** Change exit codes or which severities block.
- **Expected Outcome:** Prefer neutral phrasing ("Blocking findings" /
  `hasBlockingFindings` with alias if schema must stay) or keep the threshold
  parenthetical but drop "warnings" where it confuses. Schema rename is
  optional and must stay backward-compatible for MCP clients if changed.
- **Files:** `crates/anvil-cli/src/commands/check.rs` plain banner; optional
  JSON field alias
- **Validation:** fixture with blocking findings; human copy unambiguous.
- **Identified From:** Dave UX-2; re-triage: not a severity bug.
- **Coordinates with:** [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high — polish only.

### CIB-239: Label pre-existing tree debt in pre-commit gate (keep full-tree scan)

- **Status:** Ready
- **Priority:** P2 UX (Dave UX-3; re-triaged 2026-08-04)
- **Intent:** Gate blocks a staged-clean commit citing already-committed `.env`
  without a "pre-existing" qualifier — first-block experience blames the
  committer for a file they did not stage. Full-tree gate remains a valid
  security posture (clean the yard on first hook install).
- **Non-scope / do not:** Switch to staged-diff-only scanning without an ADR;
  that is a product/security decision, not this item.
- **Expected Outcome:** Findings on already-committed / unstaged paths are
  labeled pre-existing (or equivalent); staged new debt remains primary. Copy
  does not imply the current commit introduced the whole tree debt. Blocking
  behaviour may stay.
- **Files:** pre-commit / gate messaging (and classification if needed)
- **Validation:** committed `.env` + empty staged change still blocks with
  pre-existing qualifier.
- **Identified From:** Dave UX-3; re-triage: label, not scope shrink.
- **Coordinates with:** [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high.

### CIB-240: `tutorial` non-interactive refusal must exit non-zero with accurate copy

- **Status:** Merged 2026-08-04 via PR #3542
- **Priority:** P3 (Dave UX-4)
- **Intent:** `anvil tutorial` with no TTY (no `--no-tui` flag) refuses honestly
  but exits 0 (so `&&` chains proceed) and says "Run without `--no-tui`" even
  when that flag was not passed — cause is non-tty.
- **Expected Outcome:** Non-interactive refusal exits non-zero; message tells
  the user a TTY/interactive session is required (not "drop `--no-tui`" unless
  that flag was actually set).
- **Files:** tutorial command entry / TUI gate
- **Validation:** `anvil tutorial </dev/null` or equivalent non-tty; exit != 0;
  message without false flag claim.
- **Identified From:** Dave field report 2026-08-04 UX-4.
- **Coordinates with:** [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high.

### CIB-241: Clarify antipattern-scan name vs built-in rule catalogue scope

- **Status:** Merged 2026-08-04 via PR #3542
- **Priority:** P3 docs (Dave UX-5; re-triaged — by design, not scope creep)
- **Intent:** "antipattern-scan" is the registry-backed built-in rule catalogue,
  spanning architectural/code-quality smells, reliability rules, and the
  syntactic security-construction subset accepted by ADR-087. The broad name
  can still invite an exhaustive SAST mental model that the product does not
  implement.
- **Non-scope / do not:** Add or move rule families to "match the name"; imply
  taint analysis or exhaustive injection detection.
- **Expected Outcome:** Help / one-line description points to anvil's built-in
  rule catalogue without promising exhaustive injection/SAST coverage.
- **Files:** check help / public docs / rule family labels
- **Validation:** wording review only.
- **Identified From:** Dave UX-5; re-triage.
- **Coordinates with:** [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high after source-truth reconciliation against ADR-087 and
  `docs/architecture/checks-as-built.md`; low urgency.

### CIB-242: Optional status hint for daemon/MCP binary version skew after upgrade

- **Status:** Merged 2026-08-04 via [#3541](https://github.com/eddacraft/anvil-001/pull/3541) (`39e4dc2e4`)
- **Priority:** P3 enhancement (Dave stack note; re-triaged 2026-08-04)
- **Intent:** After a rename-swap binary upgrade, open editor/agent MCP sessions
  keep the old anvil image — normal OS process behaviour, not an anvil
  install-path bug. Multi-client fleets can accumulate stale processes; operators
  want visibility.
- **Non-scope / do not:** Auto-kill foreign MCP/editor sessions; treat long-lived
  process retention as a release defect.
- **Expected Outcome:** Best-effort: `status` (or related) discloses when a
  known daemon/MCP helper path or version differs from the CLI on PATH, with a
  restart hint. Missing process enumeration on some platforms is acceptable if
  documented.
- **Files:** status / process inventory helpers
- **Validation:** mocked process list or two versioned binaries; status shows
  skew + restart guidance when detection works.
- **Identified From:** Dave stack notes; re-triage: enhancement.
- **Coordinates with:** MCPX, multi-client, [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** medium — shape open; not a ship gate.

### CIB-243: Skill install docs — multi-client `--client` + move outside skills dir

- **Status:** Merged 2026-08-04 via [#3541](https://github.com/eddacraft/anvil-001/pull/3541) (`39e4dc2e4`)
- **Priority:** P3 docs (Dave stack notes; re-triaged 2026-08-04)
- **Intent:** (a) Requiring explicit `--client` when several clients are
  detected is **correct** non-interactive behaviour (no silent multi-write) —
  not a bug. Scripted fleets need documented enumeration. (b) Unmanaged-skill
  "move it aside" that stays under `~/.claude/skills` still shows as a live
  skill in Claude Code — copy should say move **outside** the skills directory.
- **Non-scope / do not:** Auto-install into every detected client without
  `--client` / `--all`.
- **Expected Outcome:** Help/docs state multi-client `--client` requirement;
  unmanaged-skill message says move outside the skills tree (or equivalent
  non-scanned path).
- **Files:** skill install help, error copy, public/install docs
- **Validation:** message text review; multi-client without `--client` still
  errors (behaviour preserved).
- **Identified From:** Dave stack notes; re-triage: docs/copy.
- **Coordinates with:** skill packaging, [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
- **Confidence:** high.

### CIB-244: Verdict Install section must reflect this-run selection (not only Cursor/Claude noise)

- **Status:** In Progress — flip to `Merged YYYY-MM-DD via PR #N` on merge
- **Priority:** P1 honesty / activation TUI (operator repro 2026-08-04)
- **Intent:** On the activation verdict **Install** block, the user sees
  Cursor/Claude Code skip rows (`skipped — not selected`, `skipped — already
  up to date`) rather than the MCP clients they actually chose in consent.
  Root cause: `InstallReport.per_client` is keyed by dual-era
  `McpClientId` (Cursor + Claude Code only) while multi-client consent offers
  come from the agent registry (`AgentClientId` / `registry_mcp_candidates`).
  Verdict assembly (`activation_verdict_model`) maps only
  `install_report.per_client` plus `settled_mcp` strings — so registry
  installs/selections do not appear as first-class Install rows, and
  deselected dual-era clients still dominate the list.
- **Expected Outcome:**
  - Install lists **this-run outcomes** for every client the user selected
    (installed / failed / unsafe skip), including registry clients (Grok,
    Codex, OpenCode, …), with clear labels.
  - Unselected detected dual-era clients are omitted or collapsed (e.g. one
    line "N other detected clients unchanged") — not a parade of
    `not selected` / `already up to date` that looks like "what you installed".
  - Layers MCP probes and Install stay consistent with multi-client truth
    (or Layers clearly scopes dual-era if probe still dual-only).
  - Regression test: consent selects a non-Cursor/Claude registry client →
    Install names that client; Cursor `UserDeselected` does not headline.
- **Non-scope:** Expanding live MCP probe tiers for every registry client in
  the same PR if that is a larger MCPX slice — Install honesty can land first
  with typed this-run rows from the applied consent plan.
- **Files:** `crates/anvil-cli/src/commands/start.rs` (`activation_verdict_model`,
  post-consent apply), `crates/anvil-cli/src/activation/orchestrator/`
  (InstallReport / registry apply), possibly `activation/render.rs` plain path
- **Validation:** unit/integration around `activation_verdict_model` + consent
  apply; manual `anvil start` TUI with a multi-client host selecting a
  non-Cursor client.
- **Identified From:** Operator screenshot
  `Projects/tmp/anvil-start-verdict.png` (2026-08-04) — Install shows only
  Cursor/Claude Code skips.
- **Coordinates with:** CIB-227, MCPX, ACTTUI-020, ACTMO-012,
  [#3514](https://github.com/eddacraft/anvil-001/issues/3514) (adjacent
  honesty), activation-tui
- **Confidence:** high — code path is dual-era-only; screenshot matches.

### CIB-245: Grouped multi-step consent with "what is this" on project/workflow bits

- **Status:** In Progress — flip to `Merged YYYY-MM-DD via PR #N` on merge
- **Priority:** P2 activation TUI UX (operator repro 2026-08-04; clarified same day)
- **Intent:** Consent is a **flat multi-select** of mixed write offers (project
  config, identity, witness attributes, git hooks, baseline, workflows, MCP).
  Path-only descriptions (`Write …`, `Create …`) make the **non-MCP** rows hard
  to understand — operator (product designer) did not recognise at least one
  project/workflow offer. MCP client names are comparatively clear; the
  confusing set was the **other bits**, not the MCP servers. Verdict already
  groups evidence by section (Activation / Layers / Install / Languages /
  Config); consent should be equally scannable.
- **Expected Outcome:**
  1. **Grouped consent**, not one undifferentiated list. Prefer sections
     aligned with what users think they are approving, e.g.:
     - **Project** — config seed, identity, baseline, witness attributes
     - **Hooks / git** — pre-commit / pre-push (if offered)
     - **Workflows** — GitHub (or other) workflow writes
     - **MCP clients** — per-client wiring (can stay denser)
  2. **Multi-step / multi-screen is allowed and preferred** if one screen is
     too crowded: e.g. step through Project → Hooks → Workflows → MCP, with a
     clear progress cue (same spirit as Preflight > Working > Consent >
     Verdict). Unticked-by-default and explicit submit still apply per step or
     at a final review.
  3. **"What is this" for every non-MCP offer at minimum:** short plain-language
     blurb (why anvil wants this write + what happens if skipped), plus path
     on expand or secondary line. Path-only is not enough for Project /
     Workflow / Hooks rows.
  4. **MCP rows:** still get a one-line class blurb when cheap (editor vs agent
     CLI + attach model), but that is secondary to (1)–(3).
  5. Blurbs are owned next to offer construction (`TuiConsentOffer` /
     `TuiProjectAction` / workflow labels), not ad-hoc in the TUI renderer, so
     plain and TUI cannot drift.
- **Non-scope:** Auto-ticking any row; full tutorials; changing which writes
  are offered; MCP install dual-era honesty (CIB-244).
- **Files:** `crates/anvil-cli/src/activation/orchestrator/mod.rs`
  (`TuiConsentOffer`, `add_tui_project_offers`, workflow/MCP offer builders),
  `crates/anvil-cli/src/commands/start.rs` (consent plan → TUI),
  `crates/anvil-tui/src/surfaces/activation/consent.rs` (grouping / multi-step
  chrome + expand)
- **Validation:** snapshot or model tests: offers group by kind/section;
  each Project/Workflow/Hooks offer has a non-path-only blurb; multi-step
  navigation reaches MCP step with prior selections retained; manual
  `anvil start` consent walkthrough.
- **Identified From:** Operator clarification 2026-08-04: confusing consent
  rows were project/other bits, not MCP servers; group like verdict; multi-
  screen OK.
- **Coordinates with:** CIB-244, ACTTUI-004, ACTTUI-015 (help bar), CIB-222
  honesty tone
- **Confidence:** high — product direction explicit after first-pass mis-scope.

### CIB-246: Align first-run tutorial path names with return-visit welcome hub

- **Status:** Merged 2026-08-04 via PR #3523 — hub tutorial entry renamed to
  the path picker's own title (`PATH_PICKER_TITLE`) and the hub now names the
  first-run marker in one line. Scope note: the hub↔picker crossing is closed
  and test-pinned, but the onboarding surface still offers a third name for
  the same object (see CIB-273), so the defect *class* is not fully closed.
- **Priority:** P2 welcome UX (operator screenshots 2026-08-04)
- **Intent:** `anvil welcome` presents two different menu taxonomies for the
  same product journey. First-run / path-select uses tutorial labels
  (`anvil's protection loop`, `Developer acceleration`, `Policy checks`, …).
  Return-visit hub uses `QuickStartOption` labels (`Review gate decision`,
  `Watch checks live`, `Learn the anvil model`, …). Same mental objects,
  different names — operator assumed two products, not first-run vs marker.
- **Expected Outcome:** Shared catalogue or explicit mapping: hub items that
  open tutorials use the **same title** as `TutorialPath::label` (or show both
  short hub title + path title). "Learn the anvil model" either renames to
  match path-select or is clearly "open path picker". Docs/help name the
  first-run marker behaviour in one line on the hub.
- **Files:** `crates/anvil-tui/src/surfaces/welcome/mod.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/mod.rs` / `paths.rs`,
  `crates/anvil-cli/src/commands/welcome.rs`
- **Validation:** snapshot comparison hub vs path-select titles; manual first
  run then second `anvil welcome` without mental rename.
- **Identified From:** `Projects/tmp/anvil-beta/` screenshots 14-00 (path
  complete / next path) vs 14-01 (return hub).
- **Coordinates with:** UJ, WOW, ACTTUI naming honesty
- **Confidence:** high — two label tables, no shared source of truth.

### CIB-247: First-run scan must not dump raw live-repo ERRORs as the tutorial "domain"

- **Status:** Merged 2026-08-04 via PR #3524 — welcome/tutorial framing landed.
- **Implementation note (2026-08-04):** took option **(b)**,
  honest framing of the live-repo scan. Options (a) and (c) both change *what
  is counted* — a sandbox domain or a `**/tests/**` exclusion would have made
  the number smaller and friendlier without making it truer, against the
  repo's no-over-claiming posture. The count now says what it counted
  (repo-wide, which rule families) and that it is not the chosen path's, and
  reports how many sit under test/fixture paths. No finding is hidden,
  discounted, or re-severitied; production `check` is untouched.
- **Priority:** P1 welcome honesty (operator 2026-08-04)
- **Intent:** First-run discovery / first-win re-scan of the **live** repo
  (e.g. cognition monorepo) surfaces **11 ERROR** secret findings in test
  fixtures (literal password-assignment strings and high-entropy tokens under
  `tests/` / adapter / spike paths). Post-tutorial copy says "Re-scanned your
  repo: 11 findings
  in this domain — same as when you started" after **Developer acceleration**,
  which reads as "your acceleration path found 11 bugs" rather than fixture
  noise. Operator experienced this as scary/funky during onboarding.
- **Expected Outcome:** Prefer one of: (a) first-win / path domain uses a
  controlled fixture set or sandbox for onboarding counts; (b) live-repo scan
  frames findings honestly ("secret rules on test fixtures", severity,
  suppress/test-path note); (c) default exclude `**/tests/**` / fixtures for
  first-run wow only (documented). Never imply the chosen tutorial path
  *introduced* or uniquely owns the count. Detail panel stays redacted.
- **Non-scope:** Changing secret-rule precision for production `check` (those
  findings may be correct product behaviour outside welcome).
- **Files:** `crates/anvil-cli/src/commands/welcome.rs` (discovery / first_win /
  `tutorial_state_with_scan`), first-win surfaces under
  `crates/anvil-tui/src/surfaces/tutorial/`
- **Validation:** first-run on a secret-noisy monorepo does not open with an
  unframed 11-ERROR list as the primary wow; re-scan line does not mis-attribute
  domain to the tutorial path name alone.
- **Identified From:** `anvil-beta` 13-57 findings list + 14-00 well-done
  "11 findings in this domain" after Developer acceleration.
- **Coordinates with:** WOW-005, secret check, UJ first-run
- **Confidence:** high — screenshots + first_run scan path in welcome.rs.

### CIB-248: Autoplay failures must stay in the TUI (auth root cause + recovery UX)

- **Status:** Merged 2026-08-04 via PR #3521. Expected outcomes 2 and 3 land
  in full; outcome 1 lands as "never exits the TUI" (recovery returns to the
  path picker with an explanation), **without** the richer in-path *retry /
  skip* affordances. Re-file that affordance separately if still wanted now
  the auth cause is removed.
  **Coverage correction (2026-08-04):** outcome 1 landed for the **`anvil
  welcome` entry point only**. The standalone `anvil tutorial --autoplay`
  path (`crates/anvil-cli/src/commands/tutorial.rs:150-153`) still returns
  `Err` and leaves the TUI on autoplay failure. Tracked as CIB-271; the
  wording above should not be read as a general guarantee.
- **Approach:** ADR-080's in-process posture (option (a)/(c) family) — the demo
  calls the check directly and the licence gate is never consulted. No bypass
  env var, no credential pass-through, no ADR amendment required.
- **Priority:** P0 welcome reliability (operator 2026-08-04; root cause pinned)
- **Intent:** Autoplay hits `Authentication required` mid-tutorial. Today
  `welcome.rs` treats `take_autoplay_failure()` as a **hard abort**: drop
  sandbox, `return Err(anyhow!(failure))`, leave the TUI, print
  `Error: autoplay command failed: …` on scrollback. Operator screenshot
  sequence is definitive: welcome fails → `anvil auth login` **succeeds**
  (creds written to `~/.config/anvil/credentials.json`) → welcome again →
  **identical** autoplay auth failure. Login did not fix it — so this is not
  "user was signed out" and not a flaky credential write.
- **Root cause (pinned):** `autoplay_process` in
  `crates/anvil-tui/src/surfaces/tutorial/executor.rs` spawns
  `current_exe() check <target>` with **`env_clear()`** and redirects
  `HOME`, `XDG_CONFIG_HOME`, `ANVIL_HOME`, etc. into the **sandbox** tree
  under the autoplay temp root. Real credentials live under the host
  `~/.config/anvil/`; the child never sees them. Host login cannot fix
  autoplay until isolation is reconciled with auth (by design today, wrong
  for ADR-080 demo). Fresh TTY exits cleanly on the fatal path; messy
  sessions can look "corrupted" after alt-screen leave — secondary.
- **Expected Outcome:**
  1. **Stay in TUI on autoplay failure** — in-path error with *retry* /
     *skip* / *leave autoplay* / *back to path picker*. Do **not**
     `return Err` from the welcome tutorial loop solely for autoplay fail.
  2. **Make autoplay auth-correct under isolation** — choose and document
     one of: demo-safe bypass for sandbox autoplay `check` (preferred for
     ADR-080); pass-through / inject host credentials into the sandbox env
     without breaking fixture isolation; or in-process check without a
     gated CLI re-entry. After host login, autoplay must not fail
     NotAuthenticated for sandbox demo steps.
  3. Auth-related failure copy must **not** only say "run auth login" when
     the child uses sandbox env that ignores host credentials — name that
     the demo runs isolated, or fix isolation so login actually helps.
- **Non-scope:** Weakening auth for ordinary non-sandbox `anvil check`.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/executor.rs`
  (`autoplay_process` env_clear + sandbox HOME/XDG),
  `crates/anvil-cli/src/commands/welcome.rs` (`take_autoplay_failure` → Err),
  tutorial `fail_autoplay`, CLI auth gate for child `check`
- **Validation:** signed-out and signed-in welcome both complete autoplay
  protection-loop demo without NotAuthenticated; after host login, re-run
  still succeeds; injected autoplay failure stays in TUI.
- **Identified From:** `anvil-beta` 14-03 (fail → login ok → fail again);
  operator reiteration that post-auth failure is the smoking gun.
- **Coordinates with:** ADR-080, AUTH, CIB-249, CIB-221
- **Pack-02 absorb (TUI-9):** Dave pack-02 `tutorial --autoplay` returns
  Authentication required while host `auth whoami` succeeds — same sandbox
  `env_clear` root cause. Do **not** re-file; land under this item / PR #3521.
- **Confidence:** high — screenshot sequence + source-level env isolation.

### CIB-249: Keep clean TTY teardown when welcome *does* exit (secondary to stay-in-TUI)

- **Status:** Done 2026-08-04 — superseded by CIB-248 (same root-cause family:
  the reported "crash" was the autoplay auth `Err` path, and the residual
  teardown expectations below are carried by CIB-248's stay-in-TUI work).
- **Summary:** Folded into CIB-248 so the recovery UX and the teardown
  guarantee are fixed together rather than half-fixing either surface. Clean
  teardown on intentional exit already holds on a fresh TTY; the expectations
  below stand as acceptance criteria for CIB-248, not as separate work.
- **Superseded by:** CIB-248
- **Priority:** P3 reliability (operator 2026-08-04; demoted after clarification)
- **Intent:** Initial report looked like a hard crash + session corruption.
  Clarification: the exit was the **autoplay auth `Err` path** (CIB-248); a
  **fresh terminal still exits but cleanly**. Messy prompt/ANSI was likely the
  outer session (tmux / prior noise) not handling alternate-screen leave, not a
  missing teardown on the happy error path. Primary fix is **do not exit**
  (CIB-248). This item only covers residual hygiene when welcome legitimately
  ends (quit, Ctrl-C, unrecoverable error).
- **Expected Outcome:** Intentional welcome/tutorial exit always leaves cooked
  mode / primary screen / cursor visible before scrollback errors print.
  Panic hook best-effort restore remains nice-to-have. No requirement to treat
  autoplay auth as process-fatal once CIB-248 lands.
- **Files:** `crates/anvil-cli/src/commands/welcome.rs` teardown pairing,
  shared TUI setup/teardown
- **Validation:** autoplay failure after CIB-248 does not exit; intentional
  quit leaves a clean prompt on a fresh TTY (already true today — keep a
  regression if cheap).
- **Identified From:** operator clarification on CIB-249 vs CIB-248.
- **Coordinates with:** CIB-248 (owns recovery UX)
- **Confidence:** high on demotion; teardown already OK on clean TTY.


## Pack-02 intake (Dave commissioning + TUI, 2026-08-04)

Sources: `/tmp/anvil-dave-pack02/report.md` + `report.json` (Windows 11, Git Bash,
git 2.55.0.windows.3, anvil 0.9.2-beta x86_64-msvc). **One platform / one git /
one console width** are binding scope limits.

**Baseline preserved:** Pack-01 disposition (Grok 6.1 → CIB-228..243 / #3514;
auth wall excluded; UPD-3→229; open admission intentional; trust fields keep
contract semantics). Pack-02 does **not** re-file those IDs.

**RETRACT-1 (binding):** Pack-01 verified-good "hooks install then committing a
secret is blocked" was unconditional. It holds only for **file-mode** hooks and
**extensions gate scans**. Replacement anchors (no standalone CIB — **CIB-250
was held free after collision closeout, then **claimed by pack-03 CIB-250**
(tutorial safety chain — not lint-staged, not RETRACT-1). Do not reassign):
- **Anchor A** (file-mode + scanned extension, e.g. `.env` asserts block) lives
  with **CIB-251** validation (file-mode path stays green; do not treat
  `--config` as covered without a verified hook fire).
- **Anchor B** (file-mode + `.py` / gate-miss extension documents current gap)
  lives with **CIB-255**; flips green when gate/check domain disclosure or
  domain expansion is settled.

### Pack-02 disposition map (stable Dave IDs)

| Dave ID | Disposition | Tracking |
| --- | --- | --- |
| RETRACT-1 | correction / conditioned anchors (no standalone CIB) | **Absorbed** into **CIB-251** (Anchor A / file-mode) + **CIB-255** (Anchor B / gate domain); pack-03 uses **CIB-250** for tutorial safety chain (unrelated) |
| HOOK-1 | net-new honesty (opt-in `--config`; one git) | **CIB-251** Ready P1 |
| WS-1 | net-new trust (false success) | **CIB-252** Ready P0 · coords CIB-160 |
| STATUS-1 | net-new honesty | **CIB-253** Ready P1 |
| STATUS-2 | absorbed into CIB-235 | CIB-235 pack-02 extension |
| START-1 | net-new honesty | **CIB-256** Ready P2 |
| WATCH-1 | net-new trust (ND caveat) | **CIB-254** Ready P0 |
| INIT-2 | net-new honesty | **CIB-257** Ready P2 |
| TUI-8 | net-new copy honesty | **CIB-259** Ready P2 |
| GATE-1 | net-new domain disclosure | **CIB-255** Ready P1 · coords CIB-234 |
| CHECK-1 | net-new domain/scoping | **CIB-255** (same item) |
| GATE-2 | needs internals; same family | **CIB-255** observation |
| TUI-1 | net-new polish (operator-observed) | **CIB-265** Proposed |
| JSON-1 | net-new | **CIB-262** Ready P3 · coords CIB-240 |
| PUSH-1 | needs reproduction (plausible only) | **CIB-267** Proposed |
| INIT-1 | net-new polish | **CIB-263** Ready P3 |
| INIT-3 | net-new polish | **CIB-257** (with INIT-2) |
| WELCOME-1 | net-new polish | **CIB-260** Ready P3 |
| STATUS-3 | net-new polish | **CIB-264** Ready P3 |
| PATH-1 | absorbed | CIB-237 |
| TUI-2 | net-new scoping | **CIB-258** Ready P1 |
| TUI-3 | plausible | deliberate non-scope tonight · note only |
| TUI-4 | net-new Windows | **CIB-261** Ready P2 |
| TUI-5/TUI-6 | width-dependent plausible | deliberate non-scope tonight |
| TUI-7 | net-new polish | **CIB-266** Ready P3 |
| TUI-9 | absorbed | CIB-248 / PR #3521 |
| TUI-10 | plausible low | deliberate non-scope tonight |
| TUI-N1/N2 | nitpicks | deliberate non-scope |
| R1..R8 | suggestions | deliberate non-scope (not release work) |
| Pack-01 UPD/TRUST/UX/CONF/AUTH | preserve prior disposition | CIB-228..243; AUTH untracked |

### CIB-250: Tutorial safety chain — esc-as-back → forced resume → wrong-repo activate (pack-03)

- **Status:** Ready
- **Priority:** P0 for `v0.9.3-beta` (first-time walkthrough safety — wrong-repo
  irreversible activate)
- **Intent:** Four small behaviours form **one causal chain** that walks a
  first-time user onto baseline activation aimed at the **wrong repo**, with no
  decline path (Dave pack-03 §3, Windows PowerShell, hand-driven as a beginner):
  1. Help bar labels `esc` as **back**, but `esc` **quits** the whole tutorial
     (pack-02 TUI-1 / former CIB-265).
  2. Mid-path there is **no route back to the path menu**; the escape hatch
     (`anvil tutorial --reset`) is documented only in `--help` (pack-02 TUI-3).
  3. Re-open lands on global resume — **"Press enter to activate in this repo"**
     in whatever folder the user is in now, not the path origin (pack-02 TUI-2 /
     former CIB-258: `~/.anvil/tutorial-progress.json` unscoped).
  4. That activation step has **no "writes X" disclosure** and **no decline** —
     only "press enter" — and rewrites the repo baseline.
- **Expected Outcome:** Fix as one chain, not partials —
  - Label/behaviour match for esc on tutorial chrome (`back` goes back **or**
    label says quit).
  - User can always reach the path menu without `--help` archaeology.
  - Progress is repo/workspace-scoped **or** resume always re-confirms the
    absolute target path before any write.
  - Activation step uses existing blast-radius tags (`[writes to your repo]`)
    and offers decline/skip; wrong-tree activate without confirmation is a fail.
- **Non-scope:** Redesign of the whole 60-second curriculum (see pack-03 §4
  deliberate non-scope / editorial later); other esc contexts outside tutorial.
- **Files:** tutorial chrome / help bar, progress store
  (`tutorial-progress.json`), resume landing, activation step consent.
- **Validation:** (a) esc labelled back does not silent-quit into wrong-repo
  activate; (b) complete path in repo A, `cd` to repo B, reopen tutorial → no
  activate-into-B without naming B and offering decline; (c) activation step
  shows write disclosure.
- **Identified From:** Dave pack-03 field report 2026-08-04
  (`/tmp/dave-beta-report-3.md` §3); absorbs pack-02 TUI-1/TUI-2/TUI-3.
- **Supersedes:** CIB-258, CIB-265
- **Coordinates with:** CIB-261 (Windows re-run snag on resume path), CIB-246,
  CIB-245 blast-radius tags
- **Confidence:** high — operator-observed chain; file shape for progress
  previously confirmed.

### CIB-251: Config-mode hooks honesty on doctor/status (HOOK-1)

- **Status:** Ready
- **Priority:** P1 honesty (default file-mode still works; opt-in path)
- **Intent:** On Dave's Windows git 2.55, `hook.<event>.command` config does not
  fire on commit, yet after `anvil hooks install --config` doctor reports
  `hooks-installed` pass and status shows L3/L4 on. End-to-end: secret in
  `.env` lands under `--config`, blocked under default file-mode.
- **Caveats (binding):** `--config` is **opt-in**; default file-mode works.
  One platform / one git only — may be environment-scoped if other gits honour
  config hooks. Do not claim a universal git bug without multi-platform repro.
- **Expected Outcome:** doctor/status either (a) verify a hook actually runs
  before "pass"/"on", or (b) label config-mode as installed-but-unverified /
  environment-dependent and not "L3 on". Prefer honest degraded wording over
  silent green. Optional: diagnose husky-via-Node remedy pressure that pushed
  Dave to `--config` on Node-less hosts (copy only unless product owns a
  non-Node path).
- **Non-scope:** Shipping a different git; flipping default away from file-mode.
- **Validation:** fresh repo `hooks install --config` then commit with marker
  file / secret; surfaces match whether hook fired. File-mode path remains
  green.
- **RETRACT-1 Anchor A (absorbed):** Any pack-01-derived "hooks install → secret
  blocked" regression must name **file-mode** hooks and a **gate-scanned**
  extension (e.g. `.env`). Do **not** treat `--config` install as covered by
  Anchor A without a platform-verified hook fire. Suite comments/fixtures must
  not use an unconditional "hooks install then secret blocked" claim.
- **Identified From:** Dave pack-02 HOOK-1; RETRACT-1 Anchor A absorbed 2026-08-04
  (CIB-250 left free — reserved by concurrent Claude lane).
- **Coordinates with:** CIB-255 (RETRACT-1 Anchor B / gate domain), GHOOK archive,
  CIB-235 layer honesty
- **Confidence:** high on observation; medium that root cause is anvil vs git.

### CIB-252: Workspace register must not report success when list is empty (WS-1)

- **Status:** Ready
- **Priority:** P0 for `v0.9.3-beta` (trust / durable worktree protection)
- **Intent:** With daemon running, `anvil workspace register "$PWD"` prints
  `Registered <path>.` exit 0, then `workspace list` shows none. Same via
  `anvil start` lifecycle. Blocks establishing durable worktree protection
  (upstream of PUSH-1 partial activation). Daemonless path already says
  "Daemon unavailable — not registered" — that honesty must reach the
  daemon-attached path when registration does not stick.
- **Likely related:** CIB-160 (Windows durable membership fail-closed over wire
  without peer-exe authorisation). Even if durable register is unavailable on
  Windows, **false success is worse than an explicit failure**.
- **Expected Outcome:** Register either persists and `list` shows the entry, or
  exits non-zero / prints the existing daemon-unavailable-style failure with
  reason (e.g. durable claim downgraded, admission open decorative — see
  CIB-232). Never claim Registered when the registry is empty.
- **Non-scope:** Full portable peer-exe durable membership (owned by CIB-160)
  can land separately once honesty is fixed.
- **Validation:** daemon up → register → list non-empty **or** explicit fail;
  daemon down keeps honest unavailable message.
- **Identified From:** Dave pack-02 WS-1 (Windows; requires daemon).
- **Coordinates with:** CIB-160, CIB-232, PUSH-1/CIB-267
- **Confidence:** high on false-success observation.

### CIB-253: `status` must not contradict live daemon (STATUS-1)

- **Status:** Ready
- **Priority:** P1 honesty (low-risk probe/copy)
- **Intent:** After `intercept start`, `anvil status` reports `Daemon: not
  running` while `intercept status` and OS process table show the daemon up.
  Reproduced in three repos. Same block already has accurate
  `daemon: not attesting` phrasing two lines above.
- **Expected Outcome:** Human status agrees with intercept status on
  running/not-running, or states a scoped difference explicitly (e.g. "not
  attesting this repo" vs process not running). Prefer reusing "not attesting"
  when that is the true condition.
- **Validation:** intercept start → status and intercept status agree on process
  liveness; three-repo smoke if cheap.
- **Identified From:** Dave pack-02 STATUS-1.
- **Coordinates with:** CIB-235, CIB-242
- **Confidence:** high.

### CIB-254: Daemon-path save-time must not read as clean over live secrets (WATCH-1)

- **Status:** Ready
- **Priority:** P0 for `v0.9.3-beta` (save-time trust) — investigate before
  claiming fixed
- **Intent:** With watch daemon path default, writing an AWS secret key into a
  watched `.py` yields `stale{cross-file-resolution-needed} (0 finding(s))` or
  `--action gate` "All quality gates passed! (score: 100%)". Same file with
  `ANVIL_WATCH_DAEMON=0 --no-daemon` reports the secret. Degraded
  `stale{…}` is honest in isolation but paired with `(0 finding(s))` reads as
  clean. intercept status stayed `0 save-time active`; status never left
  `L2 save off`.
- **Caveat (binding):** One early `watch --action gate` run DID fail
  secret-detection; controlled repeat passed. Treat daemon-on path as
  **non-deterministic until explained** — do not ship a "fixed" claim on a
  single happy path.
- **Expected Outcome:** (1) When degraded/stale, do not present a clean score
  or zero findings as success without the degraded label dominating. (2)
  Daemon path detects the same class of secrets as non-daemon for the same
  write, or honestly refuses save-time attachment. (3) Document non-determinism
  if residual races remain.
- **Validation:** controlled write-while-watch with AWS fixture key; daemon and
  non-daemon; repeat ≥3 for stability.
- **Identified From:** Dave pack-02 WATCH-1.
- **Coordinates with:** DSV/watch daemon routing, CIB-255 domain if extension
  interacts
- **Confidence:** high on behaviour gap; medium on root cause.

### CIB-255: Disclose `gate` / `check --all` secret file domains (GATE-1, CHECK-1, GATE-2)

- **Status:** Ready
- **Priority:** P1 trust honesty for `v0.9.3-beta` (prefer disclosure over
  forced parity — same stance as CIB-234)
- **Intent:**
  - **GATE-1:** `check app.py` finds AWS secret; `gate` PASS 100% with no
    domain statement. Gate detects secrets in `.ts/.js/.json/.yml/.toml/.env/
    .env.local` but not `.py/.rb/.go/.java/.txt/.ini/.sh/.ps1/.md`. Per-file
    check finds all sixteen. Practical: Python/Go/shell-only secret repos get
    a green commit gate.
  - **CHECK-1:** `check --all` reports "Checked 3 file(s)" (`.js/.py/.ts`
    only) while per-file check finds secrets in all 16 extensions. Correction:
    not a "dotfile exclude" bug — extension scoping.
  - **GATE-2:** `gate --json` "Anti-pattern check passed: 3 files scanned" in
    a ~25-file repo with planted secrets; mechanism unknown (plan-scoping
    **refuted** by Dave). Needs internal knowledge; treat as same family.
- **Non-scope / do not:** Force gate/check count parity without product ADR
  (Dave correctly applied the CIB-234 audit ruling). Do not expand domains
  silently without stating the previous/new domain if that is the product
  choice.
- **Expected Outcome:** Prefer domain disclosure on human + JSON gate and
  `check --all` (what was scanned / skipped), matching import-boundaries
  "Skipping" honesty. If product expands gate secret domain, flip **RETRACT-1
  Anchor B** green (below). GATE-2 count either explained in output or fixed
  once mechanism is known.
- **RETRACT-1 Anchor B (absorbed):** Pack-01 verified-good "hooks install then
  secret blocked" must not pass on a file-mode hook + **gate-miss** extension
  (e.g. `.py`) without stating that gap. Suite/fixture: file-mode + `.py` (or
  other not-detected-by-gate extension) documents current domain gap and flips
  green when this item settles domain disclosure or expansion. Complements
  Anchor A on CIB-251 (file-mode + scanned extension asserts block).
- **Validation:** multi-extension secret fixture; gate and check --all name
  domain or detect consistently with stated domain; regression tests for
  disclosure strings; Anchor B fixture as above.
- **Identified From:** Dave pack-02 GATE-1, CHECK-1, GATE-2; RETRACT-1 Anchor B
  absorbed 2026-08-04 (CIB-250 left free — reserved by concurrent Claude lane).
- **Coordinates with:** CIB-234, CIB-239, CIB-251 (RETRACT-1 Anchor A)
- **Confidence:** high on GATE-1/CHECK-1; medium on GATE-2 mechanism.

### CIB-256: `start --verify` meaning must not claim writes that did not run (START-1)

- **Status:** Ready
- **Priority:** P2 honesty (same pattern as CIB-220..222)
- **Intent:** On a virgin repo, `anvil start --verify` reports
  `state: ready_restart_required` and `meaning: anvil has written the MCP
  config…` while `config: absent` / `config: defaults` — non-mutating probe
  describes a completed write. Two different `config:` keys also render under
  one label.
- **Expected Outcome:** meaning lines describe **this run's** verified state;
  if MCP was not written, do not claim it was. Distinct config keys keep
  distinct labels. Align with start honesty pass tone.
- **Validation:** virgin repo `start --verify`; no false write claims; labels
  unique.
- **Identified From:** Dave pack-02 START-1.
- **Coordinates with:** CIB-220..222, CIB-223..225
- **Confidence:** high.

### CIB-257: Init sample-scan and language-coverage honesty (INIT-2, INIT-3)

- **Status:** Ready
- **Priority:** P2 honesty
- **Intent:**
  - **INIT-2:** Init over a sole secret-bearing file prints
    `Scanned 1 file(s) (sampled…)` then tick "No warnings found in this
    sample" — sample word is honest; the tick reads as clean.
  - **INIT-3:** Repo of 17 files (md/yml/ps1/sh) gets "No source files yet —
    nothing to scan" while `start --verify` inventories Markdown (8). Prefer
    existing vocabulary: unsupported languages, not file absence.
- **Expected Outcome:** Sample/clean results cannot be skimmed as full-tree
  clear; empty-language message uses unsupported-coverage wording.
- **Validation:** secret-in-sample fixture; non-source-only fixture.
- **Identified From:** Dave pack-02 INIT-2, INIT-3.
- **Coordinates with:** CIB-247 first-run scan framing
- **Confidence:** high.

### CIB-258: Scope tutorial progress to repo/workspace (TUI-2)

- **Status:** Done 2026-08-05 — superseded by CIB-250 (pack-03 tutorial safety
  chain folds resume scope + wrong-repo activate into one causal item with
  esc-as-back and undisclosed activation).
- **Summary:** Pack-02 TUI-2 global progress footgun absorbed into CIB-250 so
  esc-back → forced resume → activate-without-decline is fixed as one chain.
- **Superseded by:** CIB-250
- **Identified From:** Dave pack-02 TUI-2; pack-03 section 3.

### CIB-259: Learning-path copy must not claim configuration the walk did not perform (TUI-8)

- **Status:** Ready
- **Priority:** P2 honesty
- **Intent:** Binary strings claim "You now have architecture enforcement
  configured." and "Your CI pipeline now runs anvil checks on every push."
  after walks that did not perform those writes. Other paths are scrupulous
  (reporting-ahead inconsistency).
- **Expected Outcome:** Path completion copy matches demonstrated steps;
  configuration claims only after actual writes or with explicit "not applied"
  framing (tutorial already has [read-only] / [writes to your repo] tags).
- **Validation:** string audit of path completion copy vs walk side effects.
- **Identified From:** Dave pack-02 TUI-8 (binary-verified).
- **Coordinates with:** R8 editorial rule (non-binding), CIB-246
- **Pack-03 note:** Dave page-3 §6 deliberately did not rule — strings may be
  true if the walk wrote config. Keep as "verify side effects before rewriting
  copy," not a confirmed false claim.
- **Confidence:** high on strings; medium on product intent (unverified writes).

### CIB-260: Welcome must not promise save-time that `start` does not attach (WELCOME-1)

- **Status:** Ready
- **Priority:** P3 polish
- **Intent:** Welcome closes "Next: run `anvil start` for daily save-time
  protection" while `start --verify` reports `watch: not_requested`,
  `save-time: not attached`.
- **Expected Outcome:** Next-step copy matches what start actually does by
  default, or start offers save-time when that is the promised path.
- **Validation:** welcome → start --verify copy alignment.
- **Identified From:** Dave pack-02 WELCOME-1.
- **Coordinates with:** CIB-246, CIB-254
- **Confidence:** high.

### CIB-261: Policy-checks tutorial step idempotent on Windows (TUI-4)

- **Status:** Ready
- **Priority:** P2 Windows tutorial reliability
- **Intent:** Policy-checks path uses `mkdir .anvil\policies` on Windows.
  Re-run fails with raw "already exists" exit 1; `r` retries identically. Copy
  states both OS branches — Unix `mkdir -p` not tested by Dave.
- **Expected Outcome:** Re-run succeeds or offers skip framing; no raw shell
  failure as the only UX. Prefer `mkdir -p` equivalent / exist-ok on Windows
  branch.
- **Caveat:** Report Windows copy behaviour only until Unix re-verified.
- **Validation:** run Policy-checks path twice on Windows; second pass green
  or guided skip.
- **Identified From:** Dave pack-02 TUI-4; **reconfirmed pack-03** §5 (normal
  path re-run hard-stops at step 2 of 6).
- **Coordinates with:** tutorial executor, CIB-250 (resume path lands users
  back into re-runs)
- **Confidence:** high on Windows observation.

### CIB-262: Honour `--json` for `workspace list` and `tutorial` (JSON-1)

- **Status:** Ready
- **Priority:** P3 contract polish
- **Intent:** `workspace list --json` returns prose byte-identical to text;
  `tutorial --json` returns prose and suggests dropping `--no-tui` when that
  flag was not passed. Peer commands (status, doctor, check, gate, …) honour
  `--json`.
- **Expected Outcome:** Both commands emit structured JSON or refuse with
  non-zero + accurate reason. Tutorial message must not invent `--no-tui`
  (coords CIB-240).
- **Validation:** `--json` parseable; tutorial non-tty path exit non-zero.
- **Identified From:** Dave pack-02 JSON-1.
- **Coordinates with:** CIB-240
- **Confidence:** high.

### CIB-263: Init summary must list `.gitignore` edits (INIT-1)

- **Status:** Ready
- **Priority:** P3 polish
- **Intent:** Init appends `.anvil/`, exception lock, witness chain paths to
  `.gitignore` without listing that file in the summary (Config/Plans/Checks
  only). Edits are correct; summary short. (`start` separately touches
  `.gitattributes` — do not re-attribute that to init.)
- **Expected Outcome:** Summary lists every path touched, matching tutorial
  blast-radius honesty tags.
- **Validation:** init on repo with tracked `.gitignore`; summary names it.
- **Identified From:** Dave pack-02 INIT-1.
- **Confidence:** high.

### CIB-264: `status` should not create project cache as a side effect (STATUS-3)

- **Status:** Ready
- **Priority:** P3 polish
- **Intent:** In a never-activated repo, `anvil status` alone creates
  `.anvil/cache/last-seen-version`. Read-only look leaves a trace. Dave first
  missed this under dry-run clean-room without `--touch-project-state`.
- **Expected Outcome:** Prefer no project writes for pure status, or document
  and gate the cache write behind an explicit touch / activation. Global home
  cache is fine if project tree stays clean.
- **Validation:** fresh git repo; status; no new `.anvil/` unless documented.
- **Identified From:** Dave pack-02 STATUS-3.
- **Confidence:** high.

### CIB-265: Tutorial `esc back` exits instead of going back (TUI-1)

- **Status:** Done 2026-08-05 — superseded by CIB-250 (pack-03 tutorial safety
  chain; esc labelled "back" but quitting is link 1 of the causal chain).
- **Summary:** Pack-02 TUI-1 / pack-03 section 3 step 1 absorbed into CIB-250
  with resume scope and activation disclosure.
- **Superseded by:** CIB-250
- **Identified From:** Dave pack-02 TUI-1; pack-03 section 3.

### CIB-266: Watch dashboard live timestamps should not read as stale UTC (TUI-7)

- **Status:** Ready
- **Priority:** P3 polish
- **Intent:** Watch dashboard shows bare UTC (`…Z`) on a live view; operator
  in +0800 read a 75s-old entry as eight hours stale.
- **Expected Outcome:** Relative or local time for live-monitoring views;
  absolute UTC acceptable as secondary/detail.
- **Validation:** live watch entry age vs clock.
- **Identified From:** Dave pack-02 TUI-7.
- **Confidence:** high.

### CIB-267: Pre-push silent exit 0 under partial activation (PUSH-1) — needs reproduction

- **Status:** Proposed
- **Priority:** not a ship gate until reproduced with full activation
- **Intent:** With local bare remote, pre-push hook runs (GIT_TRACE), emits
  nothing, exits 0; `l4-validate` silent; audit-chain witnessed 0/10. **Dave
  confidence: plausible only.** Activation was partial (`init` +
  `start --no-mcp`) on a worktree that never registered (**WS-1**). May be
  entirely downstream of incomplete activation rather than L4 logic.
- **Also note:** invoking `anvil hook pre-push testremote <url>` like git's
  argv fails with unexpected argument — shipped shim strips argv so not a live
  bug, but GIT_TRACE copypaste misleads operators (docs only).
- **Expected Outcome:** After reproduction with durable worktree + full
  activation + a real remote path, pre-push either blocks/validates with a
  stated reason or documents intentional pass conditions. Until reproduced,
  **do not treat as a confirmed L4 regression**.
- **Validation:** blocked on CIB-252 (and full activation); then controlled
  push matrix.
- **Identified From:** Dave pack-02 PUSH-1.
- **Coordinates with:** CIB-252, CIB-216 pre-push runtime
- **Confidence:** low–medium — plausible; caveat may explain all of it.

### CIB-250 collision closeout (2026-08-04) / pack-03 claim (2026-08-05)

- Pack-02 originally numbered RETRACT-1 as CIB-250 (#3535); corrected in #3537
  (absorb into CIB-251/255). CIB-250 was left free after that correction.
- Welcome follow-ups renumbered to **CIB-268..274** (#3536); pack-02 stayed
  **CIB-251..267**.
- **2026-08-05 pack-03:** free **CIB-250** claimed for the **tutorial safety
  chain** (esc-as-back → forced resume → wrong-repo activate). Not a redefinition
  of RETRACT-1. Pack-02 251+ numbering unchanged.

### Pack-02 deliberate non-scope (do not auto-file work)

- **TUI-3** resume mid-path (plausible) — product UX later
- **TUI-5 / TUI-6** dual help bars / next: truncation — one console width only
- **TUI-10** 6 vs 7 path counts — unconfirmed intent
- **TUI-N1 / TUI-N2** nitpicks — skip freely per Dave
- **R1..R8** 60-second walk design suggestions — not findings; not release work
- **AUTH day-zero wall** — still operator-excluded from pack-01
- **Untested surfaces** — lsp, capsule, exception, edda/ember, drift, uninstall,
  wizard, new, dashboard, real remote push, three learning paths E2E, macOS,
  Linux, other gits
### CIB-268: Autoplay worker panics are silent in the terminal, hurting debuggability

- **Status:** Ready
- **Priority:** P3 developer experience (CIB-248 verification advisory 2026-08-04)
- **Intent:** `install_panic_hook` in `crates/anvil-cli/src/tui.rs` returns
  early — printing nothing and skipping `restore_terminal` — when the
  panicking thread is `AUTOPLAY_WORKER_THREAD`. That is correct for the live
  TUI (a backtrace over the frame would corrupt it), but it means a panic in
  the in-process demo check produces **no terminal output at all**. The text
  survives only in the step's `stderr`, shown in the recovery notice, which is
  truncated framing rather than a diagnostic.
- **Expected Outcome:** Panic detail from the autoplay worker reaches somewhere
  a developer will find it without corrupting the frame — e.g. the existing
  debug log sink, `ANVIL_LOG`, or a file under the sandbox root. Suppression
  stays the default for the live frame; the information does not vanish.
- **Non-scope:** Restoring the terminal from that thread, or printing over a
  live frame.
- **Files:** `crates/anvil-cli/src/tui.rs` (`install_panic_hook`),
  `crates/anvil-tui/src/surfaces/tutorial/executor.rs` (worker + `catch_unwind`)
- **Validation:** an injected panic in the autoplay worker leaves a retrievable
  message; the TUI frame is still not corrupted.
- **Identified From:** CIB-248 independent verification advisory (PR #3521).
- **Coordinates with:** CIB-248, CIB-269
- **Confidence:** high — behaviour is explicit in the hook.

### CIB-269: Panic-hook thread-name suppression is latent coupling if the name is reused

- **Status:** Ready
- **Priority:** P3 robustness (CIB-248 verification advisory 2026-08-04)
- **Intent:** The panic hook suppresses output purely by matching the thread
  **name** `AUTOPLAY_WORKER_THREAD`. Suppression is only safe because that
  thread is wrapped in `catch_unwind` and reports failure as a demo step. Any
  future thread spawned with the same name but **without** `catch_unwind`
  would silently swallow its panic and leave the terminal unrestored — a
  failure that is invisible rather than loud.
- **Expected Outcome:** Make the coupling explicit rather than nominal. Options:
  key suppression off a thread-local or atomic set by the `catch_unwind`
  wrapper rather than the name; or assert at the single spawn site that the
  name and the wrapper are introduced together, with a comment stating the
  invariant.
- **Files:** `crates/anvil-cli/src/tui.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/executor.rs`
- **Validation:** a thread bearing the name without `catch_unwind` either
  cannot be constructed or does not get suppression.
- **Identified From:** CIB-248 independent verification advisory (PR #3521).
- **Coordinates with:** CIB-248, CIB-268
- **Confidence:** high — the invariant is real but unenforced.

### CIB-270: Recorded environmental test baseline is understated and hides real failures

- **Status:** Ready
- **Priority:** P2 validation honesty (observed across PR #3521/#3523 2026-08-04)
- **Intent:** The baseline agents are handed ("the `mcp::tools::*` daemon
  family") omits members that do fail routinely, so a genuine regression can
  be waved through as "known". Three concrete gaps:
  1. `mcp_serve_stdio_resources_read_stats_returns_contents`
     (`crates/anvil-cli/tests/mcp_serve_stdio.rs:613`) fails for the same
     no-daemon reason (`not_ready` vs `unavailable`) but sits in a separate
     integration binary, so it reads as "outside the family".
  2. `telemetry::tests::reservation_commit_enforces_one_success_per_install_per_day`
     fails under parallel runs and passes in isolation; it is a known flake
     but is not written down.
  3. `cargo test -p eddacraft-anvil` **fail-fast** exits after the lib binary
     and never reaches the integration tests, so the suite silently
     under-reports unless `--no-fail-fast` is passed.
- **Expected Outcome:** One recorded baseline that names every expected
  failure and the reason, plus the `--no-fail-fast` requirement, in the place
  agents actually read (`AGENTS.md` or the probe manifest). Better still, make
  the daemon-dependent tests skip explicitly when no daemon is present so the
  baseline is empty rather than memorised.
- **Files:** `AGENTS.md` validation section, `crates/anvil-cli/tests/`
  daemon-dependent tests, `crates/anvil-cli/src/telemetry.rs` tests
- **Validation:** a fresh agent running the documented command sees either
  zero failures or exactly the documented set, and cannot mistake a real
  failure for an environmental one.
- **Identified From:** gate runs on PR #3521 and PR #3523 (2026-08-04).
- **Coordinates with:** CIB-248, aps-probe
- **Confidence:** high — reproduced on both lanes.

### CIB-271: `anvil tutorial --autoplay` still exits the TUI on autoplay failure

- **Status:** Ready
- **Priority:** P2 welcome reliability (CIB-248 coverage gap 2026-08-04)
- **Intent:** CIB-248 made autoplay failure non-fatal for the **`anvil
  welcome`** entry point: `welcome.rs:1137-1139` calls
  `recover_from_autoplay_failure` and returns to the path picker. The
  standalone `anvil tutorial --autoplay` loop was not changed —
  `crates/anvil-cli/src/commands/tutorial.rs:150-153` still does
  `abort_autoplay_session()` then `return Err(anyhow!(failure))`, dropping the
  user to scrollback exactly as the original defect described.
- **Expected Outcome:** The `tutorial` loop recovers the same way `welcome`
  does — stay in the TUI, restore the pre-demo context, return to the path
  picker with an explanation — or the divergence is deliberately documented
  with a reason.
- **Non-scope:** The richer in-path retry / skip affordances explicitly
  descoped by CIB-248.
- **Files:** `crates/anvil-cli/src/commands/tutorial.rs` (autoplay failure
  branch), `crates/anvil-tui/src/surfaces/tutorial/mod.rs`
  (`recover_from_autoplay_failure`)
- **Validation:** an injected autoplay failure under `anvil tutorial
  --autoplay` stays in the TUI and restores the pre-demo scan context.
- **Identified From:** CIB-248 verification advisory A1; confirmed against
  `main` after PR #3521 merged.
- **Coordinates with:** CIB-248
- **Confidence:** high — verified in the merged source.

### CIB-272: Full-mode welcome can squeeze the logo at exactly `height == content_height`

- **Status:** Ready
- **Priority:** P3 welcome layout (CIB-246 verification advisory 2026-08-04)
- **Intent:** In full mode at exactly `height == content_height` (18 without a
  status line, 20 with) and any width ≥ 72, the layout constraints total one
  row more than the available area because `top_pad` is floored at 1
  (`crates/anvil-tui/src/surfaces/welcome/render.rs:98`), and the logo renders
  at 6 rows instead of 7. This is **pre-existing and unchanged** by PR #3523 —
  base and head both squeeze identically — but CIB-179's guard
  (`compact_hint_never_squeezes_logo`) only sweeps *compact* mode at width 40,
  so full mode has no equivalent height sweep protecting it.
- **Expected Outcome:** Either full mode stops squeezing at that boundary, or a
  CIB-179-style height sweep for full mode pins the current behaviour so it
  cannot silently worsen.
- **Files:** `crates/anvil-tui/src/surfaces/welcome/render.rs` (`top_pad`,
  full-mode constraints and tests)
- **Validation:** a width × height sweep over full mode asserts the logo keeps
  its intended row count, or documents the exact boundary where it cannot.
- **Identified From:** CIB-246 independent verification advisory (PR #3523),
  reproduced with a standalone ratatui layout probe.
- **Coordinates with:** CIB-179, CIB-246
- **Confidence:** high for the constraint model; the real renderer is untested
  at 80×18.

### CIB-273: Onboarding still offers a third name for the tutorial ("Explore the tutorial")

- **Status:** Ready
- **Priority:** P2 welcome naming honesty (CIB-246 verification advisory 2026-08-04)
- **Intent:** CIB-246 aligned the return-visit hub with the path picker, but
  the first-run onboarding surface was not covered.
  `crates/anvil-tui/src/surfaces/onboarding/welcome.rs:20` labels
  `OnboardingChoice::SkipToTutorial` as **"Explore the tutorial"**, and it
  routes to `OnboardingOutcome::Tutorial`
  (`crates/anvil-cli/src/commands/welcome.rs:419`) — landing on the same path
  picker the hub now calls "Choose a learning path". First-run users therefore
  still meet a different name for the object returning users meet, which is
  the exact defect class CIB-246 was filed against.
- **Expected Outcome:** The onboarding entry names the picker it opens, the
  same way the hub entry now does, and the crossing is pinned by a test
  against `PATH_PICKER_TITLE` so it cannot drift back.
- **Non-scope:** Re-litigating sentence-case vs title-case per surface; the
  existing pin is deliberately case-insensitive.
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/welcome.rs`,
  `crates/anvil-tui/src/surfaces/welcome/mod.rs` (existing pin to mirror)
- **Validation:** a test asserts the onboarding tutorial entry matches
  `PATH_PICKER_TITLE` case-insensitively, mirroring
  `tutorial_entry_is_named_after_the_path_picker`.
- **Identified From:** CIB-246 independent verification advisory (PR #3523).
- **Coordinates with:** CIB-246, UJ, WOW
- **Confidence:** high — third label confirmed in source.

### CIB-274: `TutorialState::reset` drops the stashed autoplay context without restoring

- **Status:** Ready
- **Priority:** P4 latent correctness (CIB-248 review 2026-08-04)
- **Intent:** `reset` sets `autoplay_saved_context = None`
  (`crates/anvil-tui/src/surfaces/tutorial/mod.rs:1536`) rather than calling
  `restore_autoplay_context()`. That is correct **today** only because `reset`
  also clears `scan_results` and the completion fields, so there is nothing to
  restore into. It is a silent-drop shape: if `reset` ever stops clearing
  those fields, a reset mid-autoplay would permanently discard the user's
  pre-demo scan results with no error and no test failure.
- **Expected Outcome:** Make the intent explicit — either call
  `restore_autoplay_context()` before clearing, or comment the invariant that
  `reset` must clear every field the stash owns, ideally with a test that
  fails if the two field sets drift apart.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/mod.rs` (`reset`,
  `stash_autoplay_context`, `restore_autoplay_context`)
- **Validation:** a test pins that the fields cleared by `reset` are a
  superset of the fields captured by `AutoplaySavedContext`.
- **Identified From:** owner review of the CIB-248 restore delta (PR #3521).
- **Coordinates with:** CIB-248
- **Confidence:** medium — not a live defect; a trap for the next change.


## Pack-03 intake (Dave start + walkthrough first-timer, 2026-08-05)

Source: `/tmp/dave-beta-report-3.md` (Windows 11 · PowerShell 5.1 · anvil
0.9.2-beta x86_64-msvc). Scope: **`anvil start`** and **`anvil tutorial`** only;
hand-driven as a first-time user. Protection "warming" set aside. Not macOS/Linux.

**Cutline:** file only **high-traffic first-time-user** defects on the normal
path. Do not open tickets for contrived edge cases or every editorial nit.
Suggestions in report §4 (curriculum shape) are deliberate non-scope for the
release claim unless pulled in as a separate editorial pass.

**CIB-250 reservation:** the free ID after pack-02 collision closeout is used
here for the **tutorial safety chain** (report §3). RETRACT-1 remains on
CIB-251/255 only.

### Pack-03 disposition map

| Dave § | Observation | Disposition | Tracking |
| --- | --- | --- | --- |
| §3 chain | esc-as-back → no menu → global resume → activate no decline | **net-new one item** | **CIB-250** Ready P0 (supersedes 258, 265) |
| §2 dual help bars | start result two bars disagree keys | net-new polish | **CIB-275** Ready P2 |
| §2 `next:` truncates | guidance line unreadable | absorb with dual bars | **CIB-275** |
| §2 Prove "the fixture" | reads as this repo | net-new honesty | **CIB-276** Ready P1 |
| §2 label clip | mid-word progress labels | deliberate non-scope (minor) | — |
| §5 Windows policy mkdir | re-run hard-stop | absorbed | **CIB-261** (reconfirmed) |
| §5 autoplay auth | symptom only | absorbed | CIB-248 / CIB-271 |
| §5 watch UTC | bare Z on live view | absorbed | **CIB-266** |
| §5 watch demo n=1 | nitpick | deliberate non-scope | — |
| §5 6 vs 7 paths | unconfirmed intent | deliberate non-scope | — |
| §4 teaching | loop not walked, etc. | deliberate non-scope (editorial) | not release claim |
| §6 config claims | unverified | already filed; soft note | **CIB-259** |
| §6 status warming | may be intentional | already filed | **CIB-235** |
| §7 RETRACT hooks | condition anchors | already absorbed | CIB-251 / CIB-255 |
| consent/Prove/Evidence | verified good | preserve | no CIB |

### CIB-275: Start result screen — one help bar and a full `next:` line

- **Status:** Ready
- **Priority:** P2 first-run UX (every `anvil start` result screen)
- **Intent:** On the activation result screen two keyboard help bars render at
  once and disagree (arrows vs `j/k`). The `next:` guidance line truncates
  mid-sentence at the panel edge — the one line telling the user what to do
  next is unreadable. Pack-02 flagged width-dependence; pack-03 confirms on the
  normal Windows console path without seeking edge widths.
- **Expected Outcome:** A single coherent help bar; `next:` fully readable
  (wrap, scroll, or shorter line). Strings already exist in the binary for the
  full `next:` text.
- **Validation:** `anvil start` result screen on a typical console width shows
  one help bar and complete next guidance.
- **Identified From:** Dave pack-03 §2 (and pack-02 TUI-5/TUI-6 elevated from
  non-scope for normal-path confirmation).
- **Coordinates with:** ACTTUI help bar, CIB-244/245 verdict chrome
- **Confidence:** high on observation.

### CIB-276: Prove toast must not read as scanning the user's project

- **Status:** Ready
- **Priority:** P1 honesty (normal start path; first-timer misread)
- **Intent:** Prove reports "caught 1 finding(s) on the fixture …" — standing
  in a project folder, "the fixture" reads as **this repo**, so a user thinks
  their project has one issue when Prove only checked the built-in sample.
  Bounded MCP claim language is otherwise excellent; the noun is the gap.
- **Expected Outcome:** Copy names a **built-in/sample fixture** (or equivalent)
  so it cannot be skimmed as a live-repo scan. Tutorial step 1 already explains
  the term; the toast must not require that prior lesson.
- **Validation:** cold `anvil start` Prove path; no first-timer-readable claim
  that the working tree was scanned.
- **Identified From:** Dave pack-03 §2 (also pack-02 VG-2 caveat).
- **Coordinates with:** activation Prove path, CIB-222 honesty tone
- **Confidence:** high.

### Pack-03 deliberate non-scope (do not auto-file release work)

- Report **§4** curriculum / Scan·Surface·React editorial (same family as pack-02
  R1..R8) — valuable, not a ship gate for v0.9.3
- Progress label mid-word clip ("Project identi") — minor polish
- Watch demo sample size n=1 / empty queue — demo data nitpick
- Finish screen 6 vs 7 path counts — unconfirmed product intent
- Status "warming" vs five-state vocabulary — leave to **CIB-235** product call
- Config-claim paths without side-effect proof — **CIB-259** with pack-03 soft note
