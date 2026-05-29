<!--
APS Module: Continuous Improvement Backlog
==========================================
Standing intake for concrete improvement work identified across the project.
This module intentionally remains active while the project is active.
-->

# Continuous Improvement Backlog

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| CIB | —     | In Progress | 21/35    |

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

### CIB-025: Generate the index module rows so PRs don't touch the shared count

- **Status:** Proposed
- **Intent:** PRs completing a work item edit their module's `plans/index.aps.md`
  row (the `N/M` count token + curated prose); concurrent PRs collide on that
  line. CIB-022 made the count *derived* + CI-enforced but left it a hand-edited,
  textually-conflicting cell. **The observed contention is _same-module_** — four
  CIB items (CIB-017/-018/-019/-024) plus the #1995 triage all collided on the
  one `| CIB | … | N/M |` token on 2026-05-26, forcing four fully serialised
  rebase-merges. This is the deferred CIB-022 "option B", re-surfaced by real
  multi-PR contention.
- **Planning council (2026-05-27, direction-validate):** unanimous **AMEND** —
  direction valid, mechanism not Ready. Record:
  [`2026-05-27-cib-025-planning-council`](../brainstorms/2026-05-27-cib-025-planning-council.md).
- **Expected Outcome (design pass picks one shape):**
  1. **Count-only rows (cheapest).** Drop the curated prose from index rows so
     the only shared token is `N/M`, and stop hand-editing `N/M` — the per-item
     `Status:` lines (distinct lines → no cross-PR conflict) are the source and
     the count is regenerated, not written by the PR. Needs a mechanism so no PR
     ever edits `N/M` (CI/post-merge regen, or compute-on-`--check`). ~1 PR.
  2. **Fully generate the module-status table** from module files (the original
     option). A real restructure — blocked by Gates 2–4.
  3. **Per-module index fragments** concatenated by a generator.
- **Design gates (must resolve before Ready):**
  1. **Same-module mechanism — RESOLVED 2026-05-27 by
     [ADR-053](../decisions/053-advisory-aps-index-counts.md).** Generating from
     module files only *moves* the collision from the index row into the module
     file (two same-module PRs still touch the header count). Decision: feature
     PRs **never edit the `N/M` count** (they flip only their own, distinct,
     `Status:` line); the count is advisory-derived; `aps:index:check` freshness
     becomes advisory (warn); a single-writer periodic reconcile (`npm run
     aps:index`) refreshes it. Post-merge regen bot is the documented escalation
     (ADR-053 Consequences). The original validation tested the wrong case
     (different modules — passes today); corrected to same-module below.
  2. **Prose custody.** Index Progress cells carry curated narrative (PR SHAs,
     "added from session X", reparenting notes) with no structured home in module
     files (the MLP2 row alone is ~21 KB). Define where it lands, or decide to
     drop it (with the information-loss acknowledged). `docs-index.mjs` works
     because DOCGOV frontmatter is structured; APS index prose is not.
  3. **Schema heterogeneity.** The index is not one table — 13+ distinct column
     schemas across ~14 sections (Notes / Dependencies / Wave / Phase / Spec-ref
     / Surface / Tier; one section has no Progress column), and section
     membership lives only in hand-written headings. Shape 2 must declare how
     section membership + per-section columns are sourced, or restrict to the
     uniform 4-column sections.
  4. **Integrity / failure-closed.** A row generator must (a) escape `|`/newline
     in every cell (`scripts/docs/docs-index.mjs` has `escapeTable`;
     `scripts/aps/lib/modules.mjs` does **not**); (b) expose module-level *Status*
     (the parser captures only counts, not `Proposed`/`In Progress`/`Complete`);
     (c) fail loudly on an unparseable module file (no silent dropped/blank row —
     the no-silent-degrade rule); (d) skip `/archive/` rows as
     `index-counts.mjs` already does.
- **Migration:** must be waved — never a single ~107-row rewrite (that PR is the
  ultimate conflict magnet, an acute form of the disease it cures). Stage:
  additive generator first (rows frozen), then batched relocation, then cutover.
- **Validation (corrected):** two branches completing items **in the same
  module** rebase/merge with zero conflict on the count; the generated/auto count
  matches module sources; `aps:index:check` green; a fixture with `|` in a module
  title does not corrupt the table; an unparseable module fails the generator
  loudly.
- **Identified From:** CIB-019/-024 session 2026-05-26 (4 serialised rebases);
  planning council 2026-05-27.
- **Files:** `scripts/aps/index-counts.mjs`, `scripts/aps/lib/modules.mjs`,
  `scripts/docs/docs-index.mjs` (precedent), `plans/index.aps.md`. Required
  co-changes: `scripts/aps/drift-check.mjs` (independent index regex),
  `scripts/aps/active-lint.mjs` (passes the index to the linter), and the
  `.claude/rules/aps-index.md` "single source of truth" wording (truth moves to
  module files; the index becomes a generated view needing a `GENERATED` marker).
- **Coordinates with:** CIB-022 (extends count derivation → row/cell generation),
  CIB-021 (same shared-append-file problem), CIB-023.
- **Confidence:** medium — shape 1 (count-only) is small and could go Ready on
  its own; shapes 2/3 are a real restructure gated on the four design questions.

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

- **Status:** Draft
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

### CIB-028: Add a safe post-merge worktree cleanup sweep

- **Status:** Draft
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

### CIB-030: Harden `eddacraft-tui` publish doc gate parity (PR-side `-D warnings`, all-features match docs.rs, release ordering)

- **Status:** Draft
- **Intent:** Close three latent gaps in the `eddacraft-tui` publish workflow
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
  3. **Publish workflow creates the GitHub Release only after `cargo
     publish` succeeds.** Today `publish-eddacraft-tui.yml` creates the
     `eddacraft-tui-v*` Release early in the job, so a failed publish
     (e.g. run `26549955604`) leaves a stray non-draft GitHub Release
     advertising a version that is not on crates.io. The `gh release
     create` step should move after the `cargo publish` step (and after
     tag propagation to the mirror) so a failure leaves nothing public.
- **Validation:**
  - `.github/workflows/rust.yml`'s `cargo doc` step has
    `env: RUSTDOCFLAGS: -D warnings` (or equivalent).
  - `.github/workflows/publish-eddacraft-tui.yml`'s doc step passes
    `--all-features`; `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps
    -p eddacraft-tui --all-features` is green on `main`.
  - `gh release create` step in the publish workflow is positioned after
    the `cargo publish` step and after tag propagation.
  - A deliberately broken intra-doc link in a doc-only PR fails the
    PR-side `cargo doc` gate (manual smoke).
- **Identified From:** TUIR-008 first publish attempt (run `26549955604`,
  2026-05-28) failed at the doc gate with two `broken_intra_doc_links` after
  `#2018` merged green; a stray `eddacraft-tui-v0.2.3` GitHub Release
  (`github-actions[bot]`, 2026-05-28T01:57:43Z) was left behind by the
  failed run. Surfaced while opening `#2029`.
- **Files:** `.github/workflows/rust.yml`,
  `.github/workflows/publish-eddacraft-tui.yml`,
  `crates/eddacraft-tui/src/widgets/image_pane.rs` (and any sibling
  feature-gated rustdoc lints uncovered by `--all-features -D warnings`).
- **Confidence:** high — each gap is small and well-isolated; the
  all-features lint cascade is the only unknown-scope subpoint.

### CIB-031: Scope the dependency-audit gate so Rust-only lockfile changes skip the npm Trivy audit

- **Status:** Draft
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
  `dependency-audit` requirement (which gates Trivy + `license-check`), and
  (b) a `pnpm-lock.yaml` diff still does. A Rust-only-dependency PR shows green
  `cargo-deny` and no `Dependency Audit` / `license-check` failure attributable
  to unrelated npm advisories.
- **Identified From:** SCAN-005 PR #2034 (2026-05-28). Adding `ignore` to
  `anvil-bench` dev-dependencies changed only `Cargo.lock` (no npm files), but
  the `lockfile` classification ran the Trivy `Dependency Audit` (whole-repo
  `scan-ref: '.'`), which failed on pre-existing `pnpm-lock.yaml` HIGH/CRITICAL
  vulns unrelated to the change. `cargo-deny` passed. The Trivy job is advisory
  (not a required check) so it did not block, but it is persistent red-X noise
  on Rust-only PRs and can mask a genuinely new npm advisory.
- **Files:** `scripts/ci/classify-changes.sh` (the `lockfile)` classification
  case and its path→class mapping), `.github/workflows/security.yml` (the
  `dependency-audit` Trivy job AND the `license-check` job — both gated on
  `dependency-audit-required`), `.github/actions/detect-changes/action.yml`,
  `scripts/ci/classify-changes.test.sh`.
- **Confidence:** medium — must keep the Rust audit (`cargo-deny`) and the
  npm-facing gates (Trivy + `license-check`) correctly routed; touches the
  change-classifier contract, which has its own test suite to extend.

### CIB-032: Fresh worktrees fall back to a stale global oxfmt, producing false format failures

- **Status:** Draft
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

- **Status:** Draft
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

- **Status:** Draft
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
