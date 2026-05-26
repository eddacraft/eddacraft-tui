<!--
APS Module: Continuous Improvement Backlog
==========================================
Standing intake for concrete improvement work identified across the project.
This module intentionally remains active while the project is active.
-->

# Continuous Improvement Backlog

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| CIB | —     | In Progress | 11/22    |

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

- **Status:** Draft
- **Tracking:** GH issue [#1797](https://github.com/eddacraft/anvil-001/issues/1797)
- **Intent:** `anvil check` is documented in `--help` as the planless-mode
  surface, but the dispatcher only wires the `architecture` check. The
  `.anvilrc` default written by `anvil start` lists `secret-detection`,
  `import-boundaries`, and `antipattern-scan` — none of those fire under
  `anvil check`, even though the same checks pass through `anvil gate`
  and the MCP catch path on the same files.
- **Expected Outcome:** `anvil check` runs the intersection of
  `.anvilrc`-enabled and planless-eligible checks (minimum:
  `secret-detection` + `antipattern-scan`). JSON `checksRun` reflects
  what actually ran. `--help` is updated to name the planless-eligible
  set explicitly.
- **Identified From:** [2026-05-21 new-user journey audit](../audits/2026-05-21-new-user-journey-audit.md)
  finding #3. The marquee single-file demo (`anvil check src/smelly.ts`)
  silently passes a file containing a hardcoded `sk-…` key.
- **Validation:** Fixture test in the CLI integration suite that writes a
  `.ts` file containing a literal `sk-…` token, runs `anvil check
  <file> --json`, and asserts `summary.errors >= 1` with a
  `secret-detection` rule_id.
- **Coordinates with:** CIB-009 (audit / gate consistency — same
  dispatcher gap surfaces there).
- **Confidence:** high — JSON output shows `checksRun: ["architecture"]`
  on a fresh repo with the default `.anvilrc`; gate catches the same
  finding on the same file. Dispatcher-side bug, not a check-side gap.

### CIB-009: `anvil audit` and `anvil gate` disagree on the same repo

- **Status:** Draft
- **Tracking:** GH issue [#1798](https://github.com/eddacraft/anvil-001/issues/1798)
- **Intent:** On the same workspace, `anvil audit` reports "0 issues —
  project looks clean" while `anvil gate` (default profile) reports a
  failing `secret-detection`. A new user who runs `audit` first sees a
  clean bill of health for a repo that contains a planted API key.
- **Expected Outcome:** `anvil audit` runs the same default check set as
  `anvil gate` (default profile), or its summary line is rewritten to
  name explicitly which check classes it runs and which it skips.
  "0 issues" should not silently exclude secret-detection or
  antipattern-scan output.
- **Identified From:** [2026-05-21 new-user journey audit](../audits/2026-05-21-new-user-journey-audit.md)
  finding #4.
- **Validation:** A CLI test that fixtures a `.ts` file with a hardcoded
  `sk-…` literal and asserts `anvil audit` returns non-zero issue
  count, mirroring the `anvil gate` outcome on the same file.
- **Coordinates with:** CIB-008 (same dispatcher-vs-checks
  inconsistency).
- **Confidence:** high — directly observable on a tiny demo repo.

### CIB-010: `anvil watch` first-scan emits a wall of `public-api-expansion` against existing symbols

- **Status:** Draft
- **Tracking:** GH issue [#1802](https://github.com/eddacraft/anvil-001/issues/1802)
- **Intent:** On a never-baselined repo, `anvil watch` reports every
  existing exported symbol as a "new public symbol" violation (e.g. a
  one-line `greet()` helper flagged as expanding the API surface). This
  contradicts the **"new edges only"** principle stated in
  `.claude/rules/architecture.md` and teaches a brand-new user to
  ignore Anvil's warnings before they've seen a real one.
- **Expected Outcome:** First-scan suppresses `public-api-expansion`
  until the baseline pass completes. Two reasonable shapes:
  1. Seed the baseline with every existing public symbol on first
     scan; warn only on net-new symbols added after the watcher starts.
  2. Emit a single `[baseline] established N nodes` event instead of
     per-symbol violations on the cold path.
- **Identified From:** [2026-05-21 new-user journey audit](../audits/2026-05-21-new-user-journey-audit.md)
  finding #8.
- **Validation:** Integration test that runs `anvil watch` for a
  bounded period against a fresh repo containing two exported
  functions and asserts no `public-api-expansion` violations fire
  until a new export is added during the run.
- **Out of Scope:** Behaviour of `watch` against a *seeded* baseline —
  this is specifically about the cold-path / never-baselined case.
- **Coordinates with:** WOUT (Done 6/6 / archived; this item lives in
  CIB because WOUT is closed).
- **Confidence:** high — directly observable on a fresh repo.

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
- **Coordinates with:** CIB-008 / CIB-009 (`anvil check` / `audit`
  dispatcher consistency — SARIF output must reflect the same
  finding set as JSON output, so both should be in their target
  state before SARIF lands or the SARIF will mirror the bug);
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

- **Status:** Draft
- **Intent:** Decide whether `anvil bom` belongs as a first-class
  command and, if so, in which existing module (likely AGOV or
  ADOPT) — before any APS module is filed. The Drako borrow
  assessment explicitly deferred APS filing pending a scope-guard
  pass on each slice (agents / MCP servers / policy refs /
  credential refs / controlled actions) individually.
- **Expected Outcome:** A brainstorm doc at
  `plans/brainstorms/YYYY-MM-DD-anvil-bom-surface.md` that:
  (a) lists the candidate BOM slices, (b) for each slice runs the
  scope-guard decision framework (does it feed enforcement or
  witness enrichment? if no → reject), (c) names the slot for the
  surviving slices (AGOV-NNN addition, ADOPT-007, or new module),
  (d) decides whether the BOM is a view over the witness chain or
  a separate collector (per the assessment §8 open question).
  Triage outcome closes this CIB and either files the followup
  APS item(s) or records the decline.
- **Validation:** Brainstorm doc exists; scope-guard decision is
  recorded per slice; either a new APS task ID is filed
  (under AGOV or elsewhere) or this CIB closes with a "decline"
  decision and a one-line rationale.
- **Identified From:** [2026-05-24 Drako borrow assessment](../brainstorms/2026-05-24-drako-borrow-assessment.md)
  §4 Borrow B + §8 open questions. Existing partial coverage:
  `detect_agents.rs` (5-tool inventory, ADOPT-003 Merged),
  `anvil mcp-config` (MCP server config), AGOV-007 (capability
  declaration model).
- **Coordinates with:** AGOV-007 (capability declaration upstream),
  ADOPT-003 (AI tool auto-detect — Merged), MLP2-071 (cross-session
  attribution — possible consumer of BOM data for witness
  enrichment).
- **Out of Scope:** Building the surface. This CIB is triage-only.
  Implementation begins after the brainstorm closes with a
  surviving slice list and a slot decision.
- **Confidence:** medium — the scoping decision is real and
  consequential; rushing it risks scope drift into generic asset
  management, exactly the failure mode the Drako assessment §6
  flagged.

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
  CIB-010 (`anvil watch` first-scan public-api-expansion wall —
  same first-scan-vs-steady-state class of UX gap).
- **Confidence:** high — docs / output-string change; no behaviour
  change. Lowest-risk of the three Drako borrows.

### CIB-017: Tracing on the `anvil policy eval` path

- **Status:** Draft
- **Intent:** `anvil policy eval` is the CI-gating policy primitive
  (POLENG-007) but has zero `tracing` instrumentation; a production/CI
  failure surfaces only as an anyhow chain with no structured fields.
- **Expected Outcome:** `eval::run` carries `#[tracing::instrument]` with
  `policy` / `query` fields; a `debug!` after evaluation emits input byte
  size, eval duration, and finding count; the engine emits a `warn!` on
  `RwLock` poison instead of a generic `EngineError::Input`.
- **Validation:** `RUST_LOG=debug anvil policy eval …` shows the fields;
  `cargo test -p eddacraft-anvil --test policy_eval` stays green.
- **Identified From:** POLENG full council (operations seat), 2026-05-25.
- **Files:** `crates/anvil-cli/src/commands/policy/eval.rs`,
  `crates/anvil-policy-engine/src/lib.rs`.
- **Coordinates with:** POLENG.
- **Confidence:** high — additive instrumentation, no behaviour change.

### CIB-018: `catch_unwind` at the policy-engine facade boundary

- **Status:** Draft
- **Intent:** `regorus` is a single-vendor 0.10 crate with internal
  `unwrap`/`expect`; a panic on an adversarial or malformed policy aborts the
  `anvil` process with no `--json` error envelope, breaking pipeline parsers.
- **Expected Outcome:** `Engine::add_policy` / `Engine::eval` wrap the regorus
  calls in `std::panic::catch_unwind`, converting a panic into
  `EngineError::Regorus` with the panic message; `--json` always emits a
  parseable envelope and a non-zero exit. The shared `RwLock` context is
  treated as unusable after a caught panic.
- **Validation:** a unit test feeding a panic-inducing policy asserts an
  `Err`, not a process abort; existing tests stay green.
- **Identified From:** POLENG full council (operations seat), 2026-05-25.
- **Files:** `crates/anvil-policy-engine/src/lib.rs`.
- **Coordinates with:** POLENG; related to POLENG-009 (resource bounds).
- **Confidence:** medium — `catch_unwind` across the regorus closure boundary
  needs `UnwindSafe` handling.

### CIB-019: Surface Go OPA stderr in the parity gate

- **Status:** Draft
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
