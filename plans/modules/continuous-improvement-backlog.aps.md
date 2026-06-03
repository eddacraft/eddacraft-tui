<!--
APS Module: Continuous Improvement Backlog
==========================================
Standing intake for concrete improvement work identified across the project.
This module intentionally remains active while the project is active.
-->

# Continuous Improvement Backlog

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| CIB | —     | In Progress | 29/46    |

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
- **Superseded by:** [`sarif-output`](sarif-output.aps.md) (SARIFOUT module,
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

- **Status:** In Progress
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
- **Expected Outcome:** A design pass picks one of these shapes:
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
- **Validation:** (corrected) two branches completing items **in the same
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

- **Status:** In Progress
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

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** In Progress
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

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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
    for the caller, and the command is removed from the auth-bypass set.
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
  `ANVIL_ADMIN_KEY` set, the dashboard opens with `anvil auth` down; an
  `auth_bypass`/`requires_auth` test asserts `plan dashboard` is no longer in
  the bypass set; `cli-surface.md` no longer lists it as User-explicit.
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
