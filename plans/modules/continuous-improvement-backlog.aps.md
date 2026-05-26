<!--
APS Module: Continuous Improvement Backlog
==========================================
Standing intake for concrete improvement work identified across the project.
This module intentionally remains active while the project is active.
-->

# Continuous Improvement Backlog

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| CIB | —     | In Progress | 18/24    |

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

- **Status:** Draft
- **Intent:** Make Anvil findings consumable by GitHub Code Scanning
  and the standard SARIF tool ecosystem (Sonar, DefectDojo, security
  dashboards) without bespoke adapters. Findings already exist; this
  is a pure additive output mode.
- **Expected Outcome:** `--format sarif` (or `--output sarif`) on
  `anvil check`, `anvil gate`, and `anvil audit` emits SARIF 2.1.0
  conforming to the `results[]` + `rules[]` + `locations[]` subset
  (the parts GitHub Code Scanning ingests). Baseline-suppressed
  findings render under SARIF `suppressions[]` (§3.35) so reviewers
  can see what was deliberately accepted at baseline time. The
  supported SARIF subset is pinned in the spec so the maintenance
  surface stays bounded; full SARIF 2.1.0 conformance is **not** the
  goal. Existing JSON / human output modes are unchanged.
- **Validation:** Fixture tests that emit SARIF from each of the
  three commands and validate against the SARIF 2.1.0 JSON Schema;
  round-trip smoke test that uploads the emitted SARIF to a GitHub
  Code Scanning sandbox repo and confirms findings render. `pnpm
  format:check` for any in-repo doc touched.
- **Identified From:** [2026-05-24 Drako borrow assessment](../brainstorms/2026-05-24-drako-borrow-assessment.md)
  §4 Borrow A — single highest-leverage borrow from the Drako
  ladder. Drako cited as parallel evolution, not dependency.
- **Files:** `crates/anvil-cli/src/commands/check.rs`,
  `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/commands/audit.rs`,
  `crates/anvil-cli/src/output/` (likely new `sarif.rs` module
  alongside the existing emitters).
- **Coordinates with:** CIB-008 / CIB-009 (both now **Merged** —
  `anvil check` / `audit` dispatcher consistency; SARIF output must
  reflect the same finding set as JSON, and both surfaces are now in
  their target state, so SARIF won't mirror the old bug);
  COMPLY (compliance-reporting) — SARIF is upstream of framework
  mapping, not a substitute for it.
- **Out of Scope:** Full SARIF 2.1.0 conformance (only the GitHub
  Code Scanning subset). Framework-mapped compliance evidence
  (lives in COMPLY). Runtime / proxy enforcement (out per
  `docs/vision/anvil-scope-guard.md` and the 2026-05-22 Proxilion
  decline).
- **Confidence:** high — well-scoped output mode, deterministic
  findings already exist, standard schema, low blast radius.

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

- **Status:** Draft
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

- **Status:** In Progress
- **Intent:** `scripts/bench-vs-go-opa.sh` runs `opa bench … 2>/dev/null || true`;
  on an OPA error (parse failure, crash, version skew) the script reports a
  generic "no positive measurement" with OPA's diagnostic discarded, so a
  failed parity run is hard to diagnose.
- **Expected Outcome:** capture OPA stderr to a temp file and echo it before
  the `require_pos_num` bail; document `opa bench --count` semantics and
  consider a higher count for the heavy `repo_scan` fixture (it samples ~87
  vs thousands for the light policies).
- **Validation:** run the script against a deliberately broken fixture and
  confirm OPA's error text reaches the operator; gate still PASSes on the real
  fixtures.
- **Identified From:** POLENG full council (operations + adversarial seats),
  2026-05-25.
- **Files:** `scripts/bench-vs-go-opa.sh`.
- **Coordinates with:** POLENG-008; `.github/workflows/poleng-parity.yml`.
- **Confidence:** high — diagnostics-only script change.

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
  + the `plans/index.aps.md` count token; `aps:index:check` enforces it in the
  Docs Lint CI job (exit 1 on drift). Parser shared with `drift-check.mjs` via
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
