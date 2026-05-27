# Continuous Improvement Log

This file captures lightweight session learning from agents. It is evidence, not
a backlog. Promote repeated friction or executable follow-up work to
`plans/modules/continuous-improvement-backlog.aps.md` as `CIB-NNN` items.

> **Concurrent writes:** this file is `merge=union` (see `.gitattributes`), so
> independent appends from parallel agents/worktrees merge automatically — both
> entries are kept, no conflict markers. This holds under `git merge`,
> `cherry-pick`, and the default merge-backend `git rebase` (Git ≥ 2.26); only a
> forced legacy `git rebase --apply` skips it (then just keep both entries). For
> the merge to stay clean, **always append at the end and leave a blank line
> after your entry** (and don't start it with a leading blank line), so entries
> that union together stay separated. Never rewrite or reflow existing entries in
> the same change as adding one.

## Template

```md
### YYYY-MM-DD — opencode|claude|other

- **Task:** ...
- **Outcome:** ...
- **Worked:** ...
- **Failed:** ...
- **Friction:** ...
- **Improvement:** ...
- **Follow-up:** ...
```

## Entries

### 2026-05-25 — opencode

- **Task:** Continue APSCAN parser/validator compatibility work.
- **Outcome:** APSCAN-003 reached local Done state with parser, validator, tests,
  package guidance, and APS index reconciliation updated.
- **Worked:** Red/green tests around `## Work Items` and `Outcome:` gave a small
  compatibility slice without widening into active-module migration.
- **Failed:** Full `pnpm test` remained blocked by a copied `better-sqlite3`
  native binary compiled for a different Node ABI; `pnpm rebuild better-sqlite3`
  did not repair it.
- **Friction:** Worktrunk worktree node_modules reuse can leave native addons stale
  for the active Node version.
- **Improvement:** Treat native-addon ABI mismatch as environment setup evidence,
  not product failure, once targeted and typecheck/lint/doc gates are green.
- **Follow-up:** If this recurs, document the reliable rebuild/install command for
  `better-sqlite3` in the worktree setup notes.

### 2026-05-24 — opencode

- **Task:** Add continuous-improvement closeout to repo-local dev-workflow skills.
- **Outcome:** Added explicit trigger contracts, continuous-improvement closeout
  rules, and this shared log.
- **Worked:** The active CIB APS module already existed, so the log could stay
  evidence-only instead of becoming a second backlog.
- **Failed:** Nothing substantive.
- **Friction:** The Claude skill description was much less explicit than the
  OpenCode copy, which likely contributed to agents skipping it.
- **Improvement:** Keep mandatory skill triggers concrete in frontmatter and
  repeat the trigger contract near the top of the skill body.
- **Follow-up:** Watch whether Claude still skips dev-workflow after restart; if
  yes, add a global skill or command-level reminder outside this repo-local copy.

### 2026-05-24 — opencode

- **Task:** Prepare PR for dev-workflow continuous-improvement closeout.
- **Outcome:** Quick Council caught a read-only-task edge case before PR
  publication.
- **Worked:** Running a scoped Council pass before opening the PR found a
  workflow-rule conflict that normal markdown validation would not catch.
- **Failed:** Initial wording made the continuous-improvement note mandatory even
  when a task explicitly forbids writes.
- **Friction:** The OpenCode skill surface inventory had drifted from the
  repo-local `.opencode/skills/` directories.
- **Improvement:** Mandatory closeout rules need explicit no-write exceptions,
  and skill inventories should be checked against the filesystem when touched.
- **Follow-up:** none

### 2026-05-25 — claude

- **Task:** Ship MLP2-051g (`anvil start --verify --why` verbose
  tier-evidence flag) via /dev-workflow.
- **Outcome:** PR #1909 opened; 7 new pinned tests; full workspace
  cargo test + clippy + fmt clean.
- **Worked:** APS Truth Gate caught that the spec's design was
  already fully locked by `plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`
  §"Council Verdicts" item 9, so no brainstorming or planning-council
  pass was needed before code. The MLP2-070 "validate prod wire-up
  before branching" check also ran clean (no stealth implementation).
- **Failed:** Initial `render_human_verbose` missed `McpTier::NotDetected`
  — the match was non-exhaustive and rustc caught it. Cheap.
- **Friction:** `cargo check -p eddacraft-anvil-cli` failed with
  "did not match any packages" because the crate name in
  `crates/anvil-cli/Cargo.toml` is `eddacraft-anvil`, not
  `eddacraft-anvil-cli`. The directory/crate-name divergence trips
  every fresh session.
- **Improvement:** none — the divergence is a one-off learnt-once
  fact; not worth a CIB entry.
- **Follow-up:** Run /addressing-pr-reviews on #1909 after CI + Copilot
  settle.

### 2026-05-25 — claude

- **Task:** Finish as much of POLENG as possible via /dev-workflow
  (autonomous goal). Implemented POLENG-002..007 on one branch.
- **Outcome:** 8 commits, one PR pending; engine facade (input schema,
  determinism contract + Builtin trait, builtins, ADR-002/003
  post-processing, coverage/trace) + `anvil policy eval` CLI. Workspace
  cargo test + clippy + fmt + oxfmt clean. POLENG-008 left as a separate
  follow-up (open).
- **Worked:** Batching the engine-core tasks on one branch (multi-item
  PR, matching the `feat/mlp2-014-019-059-*` precedent) avoided five
  merge-wait cycles; the in-module dependency chain collapsed into
  numeric order with 004 done before 003 so the `Builtin` trait existed
  before the concrete builtins. Mini-council (kernel + adversarial +
  code-reviewer) earned its keep.
- **Failed:** Mini-council caught a CRITICAL I'd have shipped — baselined
  `Error` findings still drove exit 1 because `exit_code` only guarded
  `!baselined` for warnings; my ADR-003 reading was warning-centric. Also
  `RegorusValue::from(serde_json::Value)` silently maps unrepresentable
  values to `Undefined`; the bridge now uses `from_value` and errors.
- **Friction:** regorus 0.10.0 exposes line coverage but **no** structured
  rule-firing trace through its public API (the internal `traces` buffer
  has no `Engine` getter), so POLENG-006's trace is the query-bindings
  surface only — documented on the task, not faked. The `eddacraft-anvil`
  vs `eddacraft-anvil-cli` crate-name divergence bit again (see prior
  entry) — `-p eddacraft-anvil`.
- **Improvement:** For a facade trait downstream crates implement, fix the
  value currency (serde_json vs engine-native) up front: `Builtin::call`
  had to gain an `&PolicyInput` ctx param in POLENG-003, churning the
  trait committed in POLENG-004.
- **Follow-up:** POLENG-008 (bench parity vs Go OPA) — executable locally
  (opa 0.60.0 + go 1.26.3 present) but a separate PR and a potential
  ADR-040-revisit trigger. Run /addressing-pr-reviews after the PR opens.
### 2026-05-25 — claude

- **Task:** Close out the remaining Attribution Pipeline v3 (ATTRIB)
  work via /dev-workflow — multi-block acknowledgements-kit drivers +
  hardening: ATTRIB-016 (deterministic expander wrap), -013 (Go), -014
  (Python), -004 (bundled-binaries); defer -009; resolve -005 (CycloneDX).
- **Outcome:** Four PRs merged to main (#1925, #1929, #1932, #1934;
  index 10→14/16). ATTRIB-005 reconciled to Deferred with a Proposed
  `supply-chain-attestation` module capturing the CycloneDX →
  dependency-mapping vision. ATTRIB-009 deferred per the operator.
- **Worked:** Per-item worktree from origin/main (reset to origin/main
  after `wt switch` — local `main` lagged each time) + TDD + a background
  code-reviewer agent in parallel with Copilot, resolve threads,
  rebase-merge, APS status-flip bundled into the feature PR. The Go/
  Python drivers cloned the ATTRIB-012 Node pattern. Network-free
  fixtures: Go local `replace` dep; bundled-binaries inline inventory;
  Python a real venv + a local package (only pip-licenses fetched).
- **Failed:** Council/Copilot caught two bugs the tests missed. (1)
  ATTRIB-004 emitted records with a `\t` separator but read them with
  `IFS=$'\t'` — tab is an IFS whitespace class, so an omitted optional
  field collapsed and shifted later columns; every fixture populated
  those fields, masking it. Fix: SOH `\001` separator + a version-omitted
  test. (2) The ATTRIB-016 test piped data into `python3 - <<'PY'`, but
  the heredoc shadows stdin so the program read EOF; pass data via a file
  arg, not the pipe.
- **Friction:** `oxfmt --check` (Docs Lint + Lint & Format) formats
  `.toml` too, not only `.md` — a prose-only README edit AND a new
  `.toml.example` both failed CI because the pre-commit hook only runs
  markdownlint on `*.md`. Run `pnpm run format:check` before pushing any
  non-code text, not just markdown. The shell here is zsh: `mapfile` is
  absent and unquoted `$var` doesn't word-split — GraphQL
  thread-resolution loops needed `while read` over a temp file. Transient
  `BLOCKED` mergeState while ci.yml skip-twin jobs queue on the
  self-hosted runner is not a failure — wait, don't re-push.
- **Improvement:** Validate task-text vs shipped-architecture before
  coding. ATTRIB-005's "CycloneDX canonical intermediate" predated the
  multi-block dispatcher (which marked it a non-goal), and the chosen
  licence scanners don't emit CycloneDX — implementing it verbatim would
  have built a rejected layer. Surfaced it; operator chose defer + the
  SCA proposal.
- **Follow-up:** SCA module is Proposed, gated on Anvil's graph layer
  ingesting a dependency graph — re-open when the graphs are firing.

### 2026-05-25 — claude

- **Task:** Ship MLP2-051i (cap MCP `query_protection_claim` IPC
  timeout at 500 ms) via /dev-workflow.
- **Outcome:** PR #1923 opened; one timing test + one constant-equality
  test; full workspace cargo test + clippy + fmt clean.
- **Worked:** TDD red-then-green via the missing `MCP_PROTECTION_CLAIM_QUERY_TIMEOUT`
  symbol forced the test to compile against the not-yet-written
  constant — cheapest possible "red" without contrived runtime
  fixtures. Council `mini` (general + adversarial) caught a real
  silent-drift risk between the MCP and activation 500 ms constants;
  pinned them with a runtime equality test rather than a brittle
  prose comment.
- **Failed:** First-pass `eprintln!` formatting matched the existing
  surrounding multi-line style, which the new
  `clippy::unnecessary_trailing_comma` lint then rejected once
  `rustfmt` collapsed the call onto one line. The cycle was
  fmt-apply → clippy-fail → strip trailing comma → re-verify; would
  have shown up in CI but cheaper to catch locally.
- **Friction:** Adversarial reviewer flagged the pre-write
  `scan_buffer` path (`request_daemon_diagnostics` /
  `read_capped_response_line`) as still vulnerable to a drip-attack
  the activation lane closed in MLP2-051f. Genuinely out of scope
  for MLP2-051i but the original code comment framing implied the
  split was a complete fix rather than a deliberate scope cut.
  Reworded the comment.
- **Improvement:** none — Council mini caught it pre-push; no
  systemic gap.
- **Follow-up:** File the `scan_buffer` wall-clock-deadline
  hardening as a new MLP2 task (candidate MLP2-051k). Run
  /addressing-pr-reviews on #1923 after CI + Copilot settle.

### 2026-05-25 — opencode

- **Task:** Run Clawpatch scan, Council-review the findings, and triage them
  into GH/APS tracking.
- **Outcome:** New audit artefact captured; GH #1926 filed; EMAIL-010 added;
  high-severity corpus confirmed covered by GH #1826.
- **Worked:** Treating the scan as a delta avoided duplicating the existing
  331-finding release-council backlog while still capturing the new broadcast
  contract bug.
- **Failed:** `pnpm aps:active-lint` still fails on unrelated
  `plans/modules/weave.aps.md` missing `## Work Items`.
- **Friction:** `plans/reviews/*` was gitignored, so local Council notes under
  that directory were invisible to normal `git status`. This PR adds a tracked
  `.gitignore` exception (`!plans/reviews/20*-clawpatch-triage.md`) so the dated
  Clawpatch triage note is committed as durable evidence.
- **Improvement:** Tracked-exception pattern — keep `plans/reviews/*` ignored by
  default and allow-list durable triage artefacts via a `!`-rule, matching the
  existing post-merge / release-council exceptions.
- **Follow-up:** Resolve EMAIL-010 / GH #1926 before relying on
  preview-token-only broadcast sends.

### 2026-05-25 — opencode

- **Task:** Start DOCGOV-009 Task 1 in a new Worktrunk worktree.
- **Outcome:** Promoted DOCGOV-009 to In Progress and drafted the owner/freshness
  rubric without editing live docs.
- **Worked:** Keeping Task 1 as a sign-off gate prevented metadata backfill from
  starting before owner defaults were explicit.
- **Failed:** Nothing substantive.
- **Friction:** Worktree setup happened after initial APS reads because the
  operator clarified the workspace requirement mid-task.
- **Improvement:** For APS execution prompts that imply implementation, confirm
  worktree placement before the first repository edit.
- **Follow-up:** none

### 2026-05-25 — opencode

- **Task:** Continue DOCGOV-009 Task 2 high-authority guide metadata backfill.
- **Outcome:** Backfilled five additional guide metadata blocks, refreshed
  indexes, and passed the docs validation gate.
- **Worked:** Deferring `docs/guides/eddacraft-autonomy-constitution.md` to Task
  4 avoided guessing on draft-operational authority.
- **Failed:** Nothing substantive.
- **Friction:** Formatter wrapping is required after hand-authored metadata tables
  with long source-path cells.
- **Improvement:** Run `pnpm format` before the final validation gate for each
  metadata batch instead of waiting for `format:check` to fail.
- **Follow-up:** none

### 2026-05-25 — opencode

- **Task:** Continue DOCGOV-009 Task 2 across remaining guide, architecture, and
  low-risk runbook metadata backfill.
- **Outcome:** Backfilled additional clear surfaces while routing unsettled owner,
  stale, and as-built path-heavy documents to later judgement/fix batches.
- **Worked:** Checking inline backtick usage before governing a runbook prevented
  new as-built/source-path validation noise.
- **Failed:** Nothing substantive.
- **Friction:** As-built and runbook docs require different batching from ordinary
  guides because body code spans can become validation inputs once metadata is
  added.
- **Improvement:** Pre-screen runbook/as-built candidates with a code-span search
  before assigning `Runbook` or `As-built` type metadata.
- **Follow-up:** none

### 2026-05-25 — claude

- **Task:** Ship MLP2-047 (pre-push end-to-end subprocess integration
  tests) via /dev-workflow.
- **Outcome:** Two Linux-gated subprocess smoke tests in
  `crates/anvil-cli/tests/pre_push_subprocess.rs` covering the
  `load_policy → Ok(None)` and `MLP2-020 VersionFloor → BelowFloor`
  ADR-038 stages end-to-end. Both green; full workspace cargo fmt +
  clippy + pnpm format:check + pnpm lint:check clean.
- **Worked:** Tracing the production stderr line backwards through
  `render_verdict` → `ErrorClass::EmbeddedFailed` arm caught the
  fixture bug before I went hunting in `load_policy`. The
  `RUST_LOG=trace` env-var probe against the binary showed only
  `cli command parsed` — the load_policy `Err(_)` arm at hook.rs:412
  silently discards the underlying anyhow chain, so the trace was
  useless for narrowing.
- **Failed:** First version-floor fixture used
  `required_anvil_version: '>=99.0.0'` modelled on the docstring at
  `anvil-l4/src/lib.rs:48` (`required_anvil_version: ">=0.6.0"`).
  `RequiredAnvilVersion::parse` uses `semver::Version::parse` (EXACT
  semver only, NOT `semver::VersionReq`), so any range prefix
  parses as `InvalidFloor` and routes through `EmbeddedFailed`. Fix:
  bare `'99.0.0'`. Docstring is misleading.
- **Friction:** `cargo test --workspace` developed sporadic failures
  (20+ tests in doctor + mcp::tools across two test binaries) after
  the new test file landed; 0 failures on the same workspace with the
  test stashed; tests pass deterministically in isolation. Pre-
  existing race in tests that mutate process-global cwd via
  `std::env::set_current_dir` — `CWD_GUARD` in doctor.rs serialises
  within one test module but doesn't synchronise across modules /
  binaries; the new subprocess-spawning tests add scheduling pressure
  that surfaces the race.
- **Improvement:** Promote the doctor/mcp cwd-race to a CIB candidate
  — `with_tempdir_as_cwd` plus the bare `set_current_dir` calls in
  `mcp/tools/validate_write.rs` need a workspace-wide mutex or a
  refactor that threads cwd explicitly. The anvil-l4 docstring
  showing `>=0.6.0` as a `required_anvil_version` value is also worth
  fixing — exact semver only.
- **Follow-up:** File CIB-NNN for the cwd-race + docstring fix. Run
  /addressing-pr-reviews on the new PR after CI + Copilot settle.

### 2026-05-25 — claude

- **Task:** /goal "deliver as much of APSCAN as possible before you are blocked
  via /dev-workflow" — close the canonical-alignment migration backlog.
- **Outcome:** All 7 remaining APSCAN items shipped (PRs #1946, #1947, #1948,
  #1949, #1953, #1954, #1955). APSCAN module is now 11/11 Done, ready for
  archive.
- **Worked:** Parallel-worktree fan-out — opened all 4 of the first-wave PRs
  concurrently while one waited on Copilot, kept context warm by drafting the
  next item in another worktree. Single resolve-threads.sh helper paid off
  every iteration. Memory rule "status flip lands in initial commit" — flipping
  each work item to `Status: Merged 2026-05-25 via PR #N` before push avoided
  the need for any follow-up reconcile PRs entirely.
- **Failed:** First APSCAN-008 spec used `Status: Approved` / `Authority:
  Proposed` — not in the docs-governance vocabulary. Should have grepped the
  governance metadata table before authoring. Caught by Copilot, fixed in
  same PR via the metadata enum (`Status: Live`, `Authority: Authoritative`).
- **Friction:** Worktree cleanup is auto-mode-denied, so seven docs/apscan-*
  worktrees are still hanging around in `wt list`. Each `gh pr merge --rebase
  --delete-branch` warns "cannot delete local branch ... used by worktree"
  but the remote delete succeeds. Operator can sweep with `wt remove` per
  branch after this session.
- **Improvement:** APSCAN-006's drift-check fix (strict-equality →
  prefix-match for `Merged` / `Released/Shipped` release-record checks)
  unmasked ~30 pre-existing `shipped-aps-without-release-record` findings
  across ADOPT/DISTRIB/INTL/EATEST/RCLI3/RMCPF/INSIGHTS that the bug was
  hiding. These are real release-record drift, all advisory, all out of
  APSCAN scope — worth a focused CIB / module-level reconcile pass.
- **Follow-up:** (a) operator: `wt remove` the 7 docs/apscan-* worktrees;
  (b) archive `plans/modules/aps-canonical-alignment.aps.md` →
  `plans/archive/modules/` once APSCAN-010 settles in a tagged release;
  (c) reconcile the unmasked `shipped-aps-without-release-record` findings
  per module owner (likely tracked under each affected module's release
  evidence, not APSCAN).

### 2026-05-25 — claude

- **Task:** Implement CIB-020 — make the anvil-tui shell-chrome version
  watermark version-agnostic so release bumps don't invalidate ~38 snapshots.
- **Outcome:** PR #1961. `VERSION` split into `cfg(not(test))` =
  `CARGO_PKG_VERSION` and `cfg(test)` = `"X.Y.Z"` placeholder; 38 snapshots
  re-accepted (footer-line-only diffs); `version_matches_workspace` replaced
  by `production_watermark_uses_cargo_pkg_version`. Tests 607/0.
- **Worked:** A one-line-ish `cfg(test)` const seam fixed all 38 snapshots at
  once — far cheaper than per-surface edits or insta regex filters (the
  watermark is rendered char-by-char with `[fg:…]` markers, so filters are
  unworkable). The "simulate a version bump → green with zero churn"
  validation directly proved the CIB acceptance criterion.
- **Failed:** none.
- **Friction:** The footer width/truncation path is only covered at the
  `eddacraft-tui` library layer; the shorter placeholder hides it from the
  anvil-tui snapshot surface. Closed by a doc-comment cross-reference (council
  minor finding), but layout-coverage visibility across the wrapper/library
  boundary is a recurring soft spot.
- **Improvement:** When a placeholder is deliberately unrepresentative
  (shorter version, fixed clock, stub id), note in-place which test at which
  layer still exercises the realistic path, so the gap stays visible.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot settle; cleanup
  worktree after merge.

### 2026-05-25 — opencode

- **Task:** Reconcile DOCGOV-009 after PR #1927 merged.
- **Outcome:** Confirmed merge evidence and prepared APS status/count updates on a
  standalone bookkeeping branch.
- **Worked:** Keeping merge bookkeeping separate avoids reopening the already merged
  metadata backfill PR.
- **Failed:** Nothing substantive.
- **Friction:** Post-merge APS status updates still require an explicit follow-up
  commit/PR because the merged implementation branch is gone remotely.
- **Improvement:** Treat post-merge APS advancement as a first-class bookkeeping PR
  step when a module status changes after merge.
- **Follow-up:** Merge the bookkeeping PR after CI and review pass.

### 2026-05-25 — claude

- **Task:** Finish ATTRIB (attribution-pipeline-v3) — land ATTRIB-009, the
  cross-repo `little-termi` starter-kit port, then reconcile anvil APS.
- **Outcome:** little-termi PR #39 merged (`634ff4c5`) — v3 kit vendored via
  `git subtree` from `eddacraft/acknowledgements-starter`, v1 hand-port
  retired, CI repointed. Attribution block byte-identical. Both repos'
  acknowledgements CI green. ATTRIB-009 → Merged; module now 15/16 (only the
  deferred ATTRIB-005 remains, rehomed to supply-chain-attestation).
- **Worked:** Establishing a clean v1 `--check` baseline in the worktree
  before switching tooling proved the kit was a faithful drop-in — the diff
  reduced to 4 marker/comment lines with the 866-line block untouched, which
  made review trivial. Reusing little-termi's existing
  `rust-core/about.{toml,hbs}` via the block's `template_path`/`config_path`
  kept the port config-only.
- **Failed:** Nothing blocking. uutils `dirname` 0.8.0 (`~/.local/bin`) no
  longer mishandles the multi-segment subtree prefix that 0.2.2
  (`/usr/bin`) still does — the documented git-subtree breakage is fixed in
  the newer build, but PATH ordering still decides which wins, so the
  `/tmp/gnu-shim` insurance is still worth pinning.
- **Friction:** `/council` is anvil-scoped, so a downstream little-termi PR
  has no first-class pre-PR review surface — relied on a focused self-review
  + little-termi CI + Copilot. Cross-repo APS has no implementation-commit to
  fold the status flip into, so a standalone anvil reconcile PR is
  unavoidable (and correct) here.
- **Improvement:** none
- **Follow-up:** Upstream kit-hardening fix for three pre-existing smells
  Copilot surfaced in the vendored kit (unused `target_array_name` in
  `expand-licences.sh`; stale `rust.yml` ref in `node-driver-preflight.sh`;
  dead no-op `if` in `go-driver-render.sh`) — fix in the canonical kit and
  let the mirror + `subtree pull` propagate. Operator: `wt remove` the
  `little-termi.attrib-009` and `anvil-001.docs-attrib-009-reconcile`
  worktrees once this PR merges.

### 2026-05-26 — claude

- **Task:** Implement CIB-022 — derive APS index progress counts from module
  files so the cross-cutting index counter stops being hand-maintained/drifting.
- **Outcome:** `scripts/aps/index-counts.mjs` (`aps:index` / `aps:index:check`);
  parser extracted to `scripts/aps/lib/modules.mjs` and shared with
  `drift-check.mjs`; enforcing check wired into the Docs Lint job; fixture test
  `test:aps-index`. Authoritative-counts approach (option A) — prose preserved.
- **Worked:** Reusing drift-check's exact parser via a shared lib guarantees the
  advisory checker and the enforcing generator agree on "done" — and scoping the
  generator to drift-check's exact set (headered modules) made `--check` pass on
  the consistent tree, so enabling enforcement didn't redden main.
- **Failed:** First pass over-reached: (1) tried to manage headerless modules
  whose index counts are curated *planned* totals, not item counts; (2) matched
  index rows by link-substring, so a `superseded by [tui-reintegration](…)`
  cross-reference in another row's prose hijacked TUIR's match. Fixed by gating
  to headered modules and matching the row's *first* (name-cell) link.
- **Friction:** The index is many tables with inconsistent layouts — some rows
  have no count column at all (GATE/ILGOV/POLFED/UCFG); surfaced as notes, left
  alone.
- **Improvement:** When building a writer that mirrors an existing read-only
  checker, scope it to exactly the checker's validated set first — divergence is
  where the false positives live.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot; cleanup worktree
  after merge. The same-module prose-conflict residue is documented in CIB-022,
  not a new item.

### 2026-05-26 — claude

- **Task:** Compact the 11 done items in the standing CIB module (the file had
  grown to 776 lines) without changing the CIB-022-derived `11/22` count.
- **Outcome:** Each done item reduced to heading + `Status:` + one-line
  `Summary:`; a convention note added under `## Tasks`. Module 776 → 493 lines.
  `aps:index:check` stays `11/22` (exit 0), drift-check 0 progress findings.
- **Worked:** Because CIB-022's generator counts by heading + `Status:`, keeping
  those two lines per item meant the count was unaffected by design — the
  "compact in place, keep count" option was chosen precisely so the just-shipped
  enforcing check didn't have to change. Verified empirically before push.
- **Failed:** Nothing substantive. Line numbers shifted as I compacted top-down,
  so I re-`grep`ed positions rather than trusting stale offsets.
- **Friction:** markdownlint isn't installed locally (session-start flagged it
  MISSING), so the `Docs Lint` markdown gate can only be checked in CI.
- **Improvement:** none — the heading+Status invariant is now documented in the
  new `## Tasks` note for future compactions.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot; cleanup worktree.

### 2026-05-26 — claude

- **Task:** "Choose the next CIB item" — picked CIB-008 (security-relevant
  `anvil check` planless bug), but verifying wire-up first revealed CIB-008/009/010
  were all already implemented and left Draft. Reconciled the cluster instead.
- **Outcome:** CIB-008 → Merged via PR #1817, CIB-009 → PR #1814, CIB-010 → PR #1816
  (behaviour fixed by WATCHUX-001), each compacted to a Summary; generator bumped
  the count 11/22 → 14/22. aps:index:check ok, drift-check 0 progress findings.
- **Worked:** The "validate prod wire-up, not spec match" memory paid for itself —
  grepping the dispatcher/`audit.rs`/`watch.rs` before committing to "implement"
  caught three stale-Draft items that were done in code, turning a build task into
  a fast reconciliation. The CIB-022 generator did the count bump automatically.
- **Failed:** Nothing. The drift existed because only 011/012 of the 2026-05-21
  audit cluster were reconciled when merged; 008/009/010 fixes shipped but their
  CIB Status was never flipped.
- **Friction:** No automated check flags "code references issue #N but the CIB item
  is still Draft" — the drift was invisible until a human/agent looked.
- **Improvement:** Possible future CIB — a check that cross-references `issue #N`
  /`fix(...)` commit trailers against open CIB `Tracking:` issues to surface
  implemented-but-unreconciled items. Not filing yet (needs a real signal it
  recurs beyond this one audit batch).
- **Follow-up:** /addressing-pr-reviews after CI + Copilot; cleanup worktree.

### 2026-05-26 — claude

- **Task:** Asked to "start DASH-001 in a worktree"; pivoted via user to scope simple, shippable TUI dashboards without json-render/AI, landing a new TDASH module.
- **Outcome:** PR #1978 — `native-tui-dashboards` (TDASH) module proposed with 4 Ready items against already-persisted `.anvil/` data; DASH-001 worktree torn down (flip reverted, DASH stays Ready).
- **Worked:** Verifying prod reality before trusting the plan — `eddacraft-tui` v0.2.2 has no json-render (it shipped web-side as `@eddacraft/render`), so TUIDASH is still blocked; `anvil plan dashboard` gave a proven native-surface precedent to generalise.
- **Failed:** Flipped DASH-001 → In Progress before confirming direction; had to revert when the user redirected to TUIDASH/native dashboards. Should have surfaced the test-infra gap + dependency check before mutating APS state.
- **Friction:** `apps/website` has no test runner at all — DASH's whole wave has no TDD path until someone seeds Vitest/RTL; this is an unstated prerequisite hiding in DASH-001.
- **Improvement:** When "start <item>" lands on a module with REVIEW/archived-dep flags or no test target, run the readiness + prod-wireup check (and a quick prerequisite scan) BEFORE the In Progress flip, not after.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot on #1978; consider a DASH readiness note that web-dashboard items need a test-infra seed item first.
- **Task:** Implement CIB-018 — catch_unwind at the policy-engine facade so a
  regorus panic on adversarial/malformed policy returns a structured error, not
  a process abort.
- **Outcome:** `Engine::guard` wraps regorus calls (add_policy / set_input_json /
  eval_query / get_coverage_report / register_builtin / coverage setters) in
  catch_unwind + a poison flag. CRITICALLY, also flipped the CLI from
  `panic = "abort"` to `"unwind"` (ADR-051) — without it the guard is inert in
  the shipped binary. Tests 37/0 + CLI 11/0; dist profile builds.
- **Worked:** A 3-seat mini-council (adversarial + kernel + general) earned its
  keep: the adversarial seat caught that `panic = "abort"` makes catch_unwind a
  no-op, which would have shipped an inert fix that LOOKS like protection — the
  exact false-confidence trap flagged the session before. Kernel seat confirmed
  the AssertUnwindSafe-is-sound-via-poison reasoning and found two unguarded
  regorus calls (coverage setters, register_builtin).
- **Failed:** First implementation was inert in production (abort profile) and
  had two unguarded call sites. Both found in review, not by me.
- **Friction:** No way to e2e-test the dist-profile catch (tests always run
  under unwind; the panic trigger is a test-only builtin not reachable via the
  CLI). Verified the mechanism under unwind + that dist compiles; documented the
  gap honestly.
- **Improvement:** When a fix depends on runtime behaviour (panic strategy,
  feature flags, profiles), check the SHIPPED build's config, not just `cargo
  test` — `cargo test` defaults to `panic = "unwind"` and silently masked that
  the release/dist profile defeats catch_unwind.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot; cleanup worktree.
### 2026-05-26 — opencode

- **Task:** Fix APS dashboard body text clipping in the TUI renderer.
- **Outcome:** Added paragraph wrapping for APS dashboard body panels and a
  regression test for long detail text.
- **Worked:** A focused Ratatui snapshot test reproduced the clipped body text
  before the implementation change.
- **Failed:** Nothing substantive.
- **Friction:** The dashboard work item is archived, so urgent polish fixes do
  not map cleanly to an active APS item.
- **Improvement:** Keep small UI polish fixes tied to regression tests when APS
  planning has already closed the original feature.
- **Follow-up:** none

### 2026-05-26 — claude

- **Task:** Implement TDASH-001 — `anvil dashboard [name]` command + read-only picker surface scaffold in anvil-tui, via dev-workflow + TDD.
- **Outcome:** feat branch with the surface (DashboardPickerState), command (catalogue/resolve/JSON/plain/TUI), and main.rs wiring; 10 surface + 8 command tests; council quick run and all 5 findings addressed.
- **Worked:** Red/green TDD on the surface; the `anvil plan dashboard` (plan_dashboard) precedent made the Surface + run_surface + TestBackend shape unambiguous. Council quick earned its keep — self-sizing the plain-text column (killed a magic `14`), a narrow-terminal compact fallback matching plan_dashboard, and a seam comment on the dropped SurfaceExit discriminant.
- **Failed:** clippy `format_push_string` (-D warnings) bit after I refactored the println loop into a String builder with `push_str(&format!())`; had to switch to `writeln!` + `std::fmt::Write`.
- **Friction:** `cargo fmt` reformatting between edits invalidated two Edit anchors (the `suffix` if-block expanded multi-line), forcing re-reads.
- **Improvement:** When building a String in a loop, reach for `writeln!`/`write!` from the start — `push_str(&format!())` is a clippy-denied antipattern under the workspace `-D warnings` gate.
- **Follow-up:** TDASH-002 (architecture-health) is unblocked — flip its catalogue entry to `available`, add a `launch` arm, and add the `dashboard/architecture.rs` surface reading `.anvil/architecture.json`.
- **Task:** Implement CIB-017 — tracing on the `anvil policy eval` path so a
  CI/prod failure is diagnosable beyond an anyhow chain.
- **Outcome:** `#[tracing::instrument]` + a structured `debug!` summary on
  `eval::run`, `warn!` on engine abnormal paths; integration test asserts the
  event under `ANVIL_LOG=debug`. Added `tracing` to policy-engine (1 Cargo.lock
  edge, no new crates; hakari clean). 12/0 + 37/0 + clippy/fmt clean.
- **Worked:** Verified the wire-up FIRST (per the last-two-sessions lesson) — the
  binary's `anvil-observability::init_tracing` honours ANVIL_LOG>RUST_LOG and the
  event surfaces as JSON. Avoided shipping latent instrumentation no subscriber
  would render. Quick council caught two real gaps (a now-unreachable poison
  warn! given CIB-018's guard; two failure paths with no structured event).
- **Failed:** First cut left the gate-relevant post_process failure arms
  uninstrumented — the exact failure class the item targets. Council caught it.
- **Friction:** The observability layer writes JSON logs to **stdout**, so
  `anvil <cmd> --json` with debug logging interleaves log lines with the
  command's JSON. Pre-existing (all commands), but it undercuts turning on
  debug logging for a `--json` gate.
- **Improvement:** none new — reinforces the verify-the-wire-up habit.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot; cleanup worktree.
  Candidate CIB (not filed, flagged to operator): observability layer should
  log to stderr, not stdout, so `--json` stays clean under debug logging.

### 2026-05-26 — claude

- **Task:** Reconcile TDASH-001 (shipped via PR #1981) — its APS work-item
  status was left In Progress after merge.
- **Outcome:** TDASH-001 → `Merged 2026-05-26 via PR #1981`; CIB-022 generator
  bumped module header + index 0/4 → 1/4. `aps:index:check` ok, drift-check
  clean.
- **Worked:** Verified prod wire-up before flipping — `Commands::Dashboard`
  enum variant + dispatch arm live in `main.rs` (not just `pub mod dashboard`),
  so the scaffold is genuinely reachable, not spec-match.
- **Failed:** Nothing. The implementation PR merged without the status flip, so
  a standalone reconcile is the correct shape (no impl commit to fold into).
- **Friction:** none.
- **Improvement:** none.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot; cleanup worktree.

### 2026-05-26 — claude

- **Task:** Implement TDASH-002 (architecture-health native dashboard) and carry the TDASH-001 → Merged reconcile in the same PR.
- **Outcome:** PR #1986 — `anvil dashboard architecture` renders `.anvil/architecture.json` (summary + violations table) via the architecture crate's `load_baseline`; `dashboard.rs` became a module dir; TDASH-001 reconciled to Merged (1/4) and TDASH-002 marked Merged via PR #1986 (2/4) *in the implementation commit*, then auto-merge armed.
- **Worked:** Applied the [[feedback_aps_status_flip_in_same_commit]] lesson this time — opened the PR, grabbed #1986, amended the module to mark TDASH-002 Merged-via-PR before merge, so no throwaway reconcile PR. Folding the prior item's reconcile into the next item's PR (per the earlier AskUserQuestion) avoided a standalone hot-file bookkeeping PR entirely.
- **Failed:** Guessed the eddacraft-tui `DataTable` API (`.headers()/.rows()/.title()` builders) — real API is `DataTable::new(theme, &headers, &rows).widths().block(Container…to_block())`. Also tripped clippy `needless_pass_by_value` on a `value: String` arg only borrowed by `format!`.
- **Friction:** Two render helpers (`metric` closure + `metric_span` fn) were redundant; collapsed to one. `cargo fmt` between edits kept invalidating Edit anchors.
- **Improvement:** Before using a sibling crate's widget builder, grep its `impl` block for the actual method set rather than inferring from the call shape — the constructor often takes the data the builders seem to.
- **Follow-up:** TDASH-003 (drift snapshots, `.anvil/snapshots/` + baseline.json) and TDASH-004 (suppressions) are the remaining items; both slot into the same `launch` arm + `commands/dashboard/<name>.rs` + `surfaces/dashboard/<name>.rs` pattern.

### 2026-05-26 — claude

- **Task:** Implement TDASH-003 — `anvil dashboard drift` native drift-snapshots
  dashboard (PR #1988).
- **Outcome:** Surface + CLI handler mirroring TDASH-002; reuses the `drift`
  command's snapshot readers (promoted to `pub(crate)`) + the architecture
  baseline loader. 9 surface + 7 CLI tests; --json/plain/TUI verified against two
  real `drift snapshot` captures. fmt + workspace clippy clean. Status flipped to
  Merged in-PR (counts 2/4 → 3/4) to avoid a reconcile PR.
- **Worked:** The TDASH-002 follow-up note in this log named the exact extension
  seam (launch arm + two files), so the implementation was pattern-fill. Reusing
  the existing `DataTable::new(...).widths().block(...)` API (the gotcha the -002
  session recorded) avoided re-discovering it.
- **Failed:** Nothing substantive. rustfmt reflowed two long fn signatures + the
  `Layout::vertical` line on first `--check`; applied and re-verified.
- **Friction:** The TDASH-003 item names `.anvil/baseline.json`, but the drift
  baseline snapshots score against is `.anvil/architecture.json` (anvil-baseline's
  `baseline.json` is a separate fingerprint store). Documented the divergence in
  the module doc comment rather than silently picking one.
- **Improvement:** none — the per-surface pattern is now well-worn; TDASH-004
  (suppressions) is the same shape against `.anvil/suppressions.json`.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot on #1988; cleanup
  worktree after merge.

### 2026-05-26 — claude

- **Task:** Council review (5 reviewers) of TDASH-003 + EAMIG-004 during the
  GitHub Actions outage, to harden both queued PRs before CI returns.
- **Outcome (TDASH-003):** Fixed `DriftDelta::trend()` i64 addition overflow
  (`saturating_add`) + added a `trend_saturates_without_overflow` test; switched
  `net()`'s post-clamp `unwrap_or(i64::MAX)` to `.expect(...)` so a future clamp
  regression is loud. 10 tui + 7 cli tests, fmt + workspace clippy green.
- **Worked:** Running all 5 reviewers in parallel against both worktrees gave
  fast triable signal; the overflow in `trend()` (MAJOR, flagged by two seats)
  was a real latent bug the unit tests hadn't exercised.
- **Failed:** Nothing in the fix. Council also flagged larger items I chose to
  defer (unbounded snapshot load, ANSI in TUI names, metric_delta vs net()
  divergence in the existing drift.rs) as out-of-scope follow-ups, not blockers.
- **Friction:** none.
- **Improvement:** Pragmatic vs Copilot disagreed on the i128 `net()` (gold-
  plating vs requested); kept it since Copilot already asked for it — don't
  re-litigate a merged review thread on a style call.
- **Follow-up:** /addressing-pr-reviews after CI on #1988; deferred items above
  if Boring-Week usage exercises them; cleanup worktree after merge.
- **Task:** Implement TDASH-004 (suppressions-overview native dashboard) via dev-workflow.
- **Outcome:** PR #1989 — `anvil dashboard suppressions` renders `.anvil/suppressions.json`; suppressions loader extracted from `commands/export.rs` to shared `services/suppressions.rs`; TDASH-004 marked Merged via PR #1989 (3/4) in the implementation commit; module now 3/4 (only TDASH-003 drift remains).
- **Worked:** Reusing the established surface+CLI pattern made this fast. Extracting the loader to `services/` rather than backwards-importing from `export.rs` or duplicating kept it DRY and gave a neutral home; council confirmed the extraction was behaviour-preserving. Applied prior lessons proactively (stable --json envelope, no phantom command, correct DataTable API) so council found 0 critical/major.
- **Failed:** clippy `map_unwrap_or` on the date-only `.map().unwrap_or_else()` — should be `map_or_else`. Same class as the TDASH-002 `needless_pass_by_value` miss: a closure idiom clippy denies under `-D warnings`.
- **Friction:** none beyond the usual fmt-invalidates-Edit-anchors.
- **Improvement:** Keep a mental checklist of clippy-denied Option/Result idioms (`map().unwrap_or_else()` → `map_or_else`; `value: String` borrowed-only → `&str`) and write them right the first time.
- **Follow-up:** TDASH-003 (drift snapshots, `.anvil/snapshots/` + baseline.json) is the last item — closes the module at 4/4.
- **Task:** Triage CIB-015 (`anvil bom` surface) while a GitHub Actions outage
  blocked all merges — scope-guard the five candidate BOM slices before filing.
- **Outcome:** Brainstorm `2026-05-26-anvil-bom-surface.md`; decline to file an
  APS item now. 3/5 slices survive as a read-only _view_ over wired collectors
  (agents cache, policy list/bundles, witness/protection summary); MCP +
  credential slices rejected (need new collectors); controlled-actions defers to
  AGOV-007. The earning hook is a `--diff` drift gate, not the inventory.
  CIB-015 → In Progress (count untouched until the Merged flip).
- **Worked:** Fanned the 6-surface map to an Explore agent, then verified its
  two over-simplifications myself before trusting them.
- **Failed:** Nothing material.
- **Friction:** Explore's "view over the witness chain" feasibility was wrong —
  `ProtectionClaim` carries no agent attribution and the witness-line
  `agent_tag` isn't reliably persisted, so the §8 chain-view option isn't
  available today; the agent slice must read the detected-agents cache.
- **Improvement:** Treat a fan-out Explore map as a lead, not a verdict — its
  per-surface wired/inert calls still need a spot-check against the actual
  producer. Its "agent_tag is an inert placeholder" verdict missed the whole
  `anvil-attribution` minting/propagation path.
- **Follow-up:** File `AGOV-NNN` (view + `--diff` drift) when AGOV leaves the
  launch parking lot and a concrete `--json`/drift consumer appears.
- **Task:** File + implement CIB-024 — route CLI tracing to stderr so
  `anvil … --json` stdout stays clean (the footgun CIB-017 surfaced).
- **Outcome:** `anvil-observability::init_tracing` routes `BinaryKind::Cli` to
  stderr (daemon + file-sink untouched); two integration tests (debug-level and
  the default-filter warn! case) assert stdout stays one clean JSON doc.
  16/23 → 16/24. fmt/clippy clean; observability 22/0; policy_eval 14/0.
- **Worked:** TDD red→green was clean — the failing test (parse `--json` stdout
  under ANVIL_LOG=debug) proved the bug on main before the one-branch writer
  fix. Council caught that I'd only tested the debug path, not the motivating
  default-filter warn! case — added that.
- **Failed:** First test covered only ANVIL_LOG=debug; the actual bug (a warn!
  at the default filter polluting --json) was unexercised until council flagged.
- **Friction:** none.
- **Improvement:** When a fix targets a specific trigger (here: warn! at the
  *default* filter), test that exact trigger, not just the easy-to-reach
  variant (debug logging).
- **Follow-up:** /addressing-pr-reviews after CI + Copilot; cleanup worktree.
- **Task:** Implement CIB-019 — surface Go OPA stderr in the parity gate
  (`scripts/bench-vs-go-opa.sh`), closing the POLENG ops-seat trio (017/018/019).
- **Outcome:** OPA stderr captured to a temp file + surfaced from
  `require_pos_num`; a `BENCH_HARNESS` override (skips the build) makes it
  testable; new stubbed fixture test (`bench-vs-go-opa.test.sh`, wired into the
  script-fixtures CI step) proves happy-path PASS + opa-error surfaces stderr,
  with no real `opa`/release build needed.
- **Worked:** The `BENCH_HARNESS` affordance turned an integration-only script
  (needs opa + a release harness) into something a hermetic stub test exercises
  in <1s. Council caught a missing assertion and raised a `BENCH_HARNESS=''`
  edge case that was inverted logic — verified empirically and dismissed (the
  suggested `+x` fix would have *introduced* the bug it described).
- **Failed:** Nothing in the code. Process friction below.
- **Friction:** A sibling PR (CIB-024) got stuck — GitHub Actions stopped
  delivering `synchronize`/`reopened` webhooks for its rebased SHAs (force-push
  amend, close+reopen, and a normal appended-commit push all left `check-runs`
  total_count=0), so auto-merge can't fire despite identical content having
  passed CI on the prior SHA.
- **Improvement:** A normal appended commit triggers CI more reliably than a
  `--force-with-lease` amend — though during an Actions delivery hiccup neither
  fires. Avoid >1 in-flight CIB PR at once: they collide on the single index
  CIB-count row, so serialize the Merged-flip.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot; flip CIB-019 Merged
  + regenerate the count only after CIB-024 lands (avoids the count race);
  cleanup worktree.
### 2026-05-26 — opencode

- **Task:** Review and patch the SCAN module after finding policy discovery was
  still on `walkdir::WalkDir` despite SCAN-001 being marked complete.
- **Outcome:** `policy test` discovery now uses the shared `ignore::WalkBuilder`
  shape, and secret-scan findings are sorted after parallel collection/dedupe.
- **Worked:** The review caught an APS/code mismatch before treating it as a
  documentation-only descoping issue.
- **Failed:** The first comparator used `u8::cmp` without borrowing the RHS;
  targeted tests caught the compile error immediately.
- **Friction:** SCAN-001 listed several scan-fanout paths, but policy's test-file
  counting path had no helper seam, so the smallest fix needed a local helper.
- **Improvement:** When an APS item claims every named call-site migrated, grep
  the exact legacy primitive during review before accepting completion wording.
### 2026-05-26 — opencode

- **Task:** Continue RCLI work by selecting an executable CLI parity slice.
- **Outcome:** RCLI2 remained OPAE-gated, so RCLI3-002 was promoted and completed
  with `anvil edda show <id>` plus targeted tests.
- **Worked:** Checking RCLI2 and RCLI3 together avoided forcing blocked OPAE work
  and found a small display-only slice over shipped RCLI3-001 YAML loading.
- **Failed:** Initial index note treated RCLI3-002 as if it had already been in
  the Ready count; reconciliation corrected the count to 5/20 with 7 Ready.
- **Friction:** RCLI3 stats were stale by phase even though item statuses were
  current enough to execute from.
- **Improvement:** When promoting one APS item opportunistically, update both the
  item and the phase/status summary in the same pass.
- **Follow-up:** none
- **Task:** Implement EAMIG-004 — expand git-history secret scan to match
  on-disk file coverage (PR #1994). Picked as unrelated queue-up work while a
  GitHub Actions outage blocked the TDASH-003 PR.
- **Outcome:** Replaced the narrow `*.ts/*.js/...` allowlist pathspec with
  `-- .` + `:(exclude)*<ext>` derived from `config.skip_extensions` (parity with
  on-disk `should_skip_file`); mirrored the SCAN-002 `max_line_bytes` guard into
  the history loop. 3 new git-repo tests; full anvil-checks suite + workspace
  clippy + fmt green.
- **Worked:** Reproducing the exact `git log -p -- . :(exclude)...` invocation
  in a throwaway repo confirmed the pathspec syntax before trusting the Rust
  wiring — separated "is git happy" from "does the parser match".
- **Failed:** First test run found nothing on the positive case. Root cause was
  the test fixture secret `AKIAIOSFODNN7EXAMPLE` — the git scanner calls the
  broad `is_allowlisted` (keyword tier incl. `example`), so the canonical AWS
  textbook key is suppressed there. It also made the exclude/guard tests *false
  passes* (empty for the wrong reason). Switched to a clean `AKIA…`-shaped key.
- **Friction:** The keyword-allowlist tiering differs between surfaces: on-disk
  exempts high-confidence shape patterns from the keyword tier (issue #1800),
  the git scanner does not. Pre-existing; flagged in the PR as out of scope.
- **Improvement:** When a fixture is a well-known dummy value, assume scanners
  may allowlist it — pick a realistic-but-synthetic needle so a green test
  proves the behaviour under test, not the allowlist.
- **Follow-up:** /addressing-pr-reviews after CI + Copilot on #1994; cleanup
  worktree after merge. Possible EAMIG item: align git-scanner allowlist tiers
  with the on-disk shape/keyword split (#1800 parity).

### 2026-05-26 — claude

- **Task:** Council review (5 reviewers) of TDASH-003 + EAMIG-004 during the
  GitHub Actions outage, to harden both queued PRs before CI returns.
- **Outcome (EAMIG-004):** Fixed four reviewer findings: (1) validate
  `skip_extensions` before embedding in a git `:(exclude)` pathspec — unsafe
  entries are skipped+warned, failing toward *more* scanning (CRITICAL,
  adversarial); (2) parse the post-image path from `+++ b/<path>` (stripping
  git's tab terminator) so filenames with spaces attribute correctly (MAJOR);
  (3) warn on non-zero `git log` exit instead of silently returning empty
  (MAJOR, ops); (4) branch the allowlist on `pattern.high_confidence` to skip
  the keyword tier for credential-shape patterns — #1800 parity, so textbook
  `AKIA…EXAMPLE` keys surface in history too (MAJOR, adversarial). +3 tests.
  Extracted `build_log_args` to stay under the 100-line clippy limit.
- **Worked:** The council folded my own pre-filed follow-up (the #1800 allowlist
  tier gap) into this PR — fixing it here makes the broadened coverage actually
  effective rather than shipping coverage that keyword-suppresses real keys.
- **Failed:** The `+++ b/` parse first captured a trailing `\t` (git's header
  terminator for space-containing paths); test caught it, fixed with
  `strip_suffix('\t')`.
- **Friction:** Adding four fixes pushed `scan_git_history` past clippy's
  `too_many_lines` (pedantic) — extracted the arg builder.
- **Improvement:** none new.
- **Follow-up:** /addressing-pr-reviews after CI on #1994. Deferred council
  items (not blockers): stream `git log -p` instead of buffering (OOM on huge
  histories), thread a `git_error` field to the caller. Cleanup worktree.

### 2026-05-27 — claude

- **Task:** Promote CIB-025 (generate index rows) toward Ready via a planning
  council, citing the prior session's 4 serialised rebases as evidence.
- **Outcome:** Council was unanimous **AMEND**, not proceed — CIB-025 → Proposed
  (not Ready). Record at `plans/brainstorms/2026-05-27-cib-025-planning-council.md`;
  item rewritten with corrected (same-module) framing, 4 design gates, and a
  waved-migration constraint. No count change (Draft→Proposed).
- **Worked:** Pre-seeding the adversarial lens with the same-module-vs-cross-
  module suspicion paid off — all four lenses independently converged on it: the
  proposed generate-from-modules mechanism only *moves* same-module contention
  into the module file, and the original validation tested a case that already
  passes. The council earned its keep by stopping a plausible-looking item from
  going Ready with a strawman acceptance test.
- **Failed:** Nothing — this was the council doing its job pre-execution.
- **Friction:** none.
- **Improvement:** When a backlog item's validation describes a *different*
  scenario than the bug that motivated it (here: "different modules" vs the
  observed same-module collisions), treat that mismatch as a direction-validate
  red flag before promoting to Ready.
- **Follow-up:** Operator to choose: split shape 1 (drop index prose →
  count-only) as a small Ready win, or a `plan-create` pass resolving Gates 1–4
  for the full restructure. Do not implement while AMEND stands.

### 2026-05-27 — claude

- **Task:** Ship DISTRIB-005 (`anvil migrate schema`) + INSIGHTS-002
  (`anvil insights --suppressions`) via /dev-workflow, queued during a GitHub
  Actions outage (work spanned 2026-05-26 → 27; merged 05-27).
- **Outcome:** PRs #1984 + #1996 merged (DISTRIB 5/5; INSIGHTS 2/4). Both
  truth-validated first: DISTRIB-005's `migrate.rs` already existed (MLP2-040 →
  subcommand split); INSIGHTS-002 had no suppression log (live antipattern scan
  instead). This reconcile PR flips both to Merged + re-adds these notes.
- **Worked:** APS truth-validation before code caught stale specs in both (named
  files/surfaces that didn't exist) — same class twice. Council quick caught real
  bugs pre-PR (ad-hoc ignore list hiding `packages/anvil/`; basename-collision
  masking stale suppressions).
- **Failed:** Skipped the addressing-pr-reviews loop on #1996/#2001 before
  declaring them "queued" — the `required_review_thread_resolution` ruleset
  then blocked merge on unresolved Copilot threads (several real: directive-vs-
  Warning.suppressed source, wrong `anvil baseline --refresh` command, wrong
  ledger record shape). Cost a full extra fix+resolve round per PR.
- **Friction:** `merge=union` CI-log shows CONFLICTING in GitHub's mergeability
  preview during a frozen-main window (Actions outage) even though it auto-merges
  — burned several rebase/force-push cycles on #1984 before I pulled the CI-log
  note out of the feature PRs entirely (re-added here).
- **Improvement:** Always run addressing-pr-reviews (incl. waiting for Copilot)
  BEFORE calling a PR "queued/done" — opening + arming auto-merge is not the end
  of the loop; thread-resolution is a hard merge gate here. And keep the CI-log
  out of feature PRs; batch its notes into the bookkeeping reconcile PR.
- **Follow-up:** `wt remove` the DISTRIB-005, INSIGHTS-002, ADR-052 worktrees;
  INSIGHTS-003 + the ADR-052 auto-snapshot capability remain (blocked on ADR
  acceptance).

### 2026-05-27 — claude

- **Task:** Planning council on the ADR-052 drift-snapshot trigger (operator
  wanted alternatives to scheduled-CI before accepting).
- **Outcome:** ADR-052 revised (PR #2001, Proposed) from weekly-CI-snapshot to an
  append-only **edge-delta event ledger** appended on merge-to-main; matches
  INSIGHTS-003's actual spec source, lossless vs weekly sampling, carries
  `anvil_version`/`rules_sha`.
- **Worked:** The council reframed "which trigger" into "what to capture" — the
  architect spotted that INSIGHTS-003's spec names `baseline diff entries`, not
  snapshots, and the adversarial pass found the determinism gap (no version/rules
  in `DriftSnapshot`). The edge-delta model resolves both by construction.
- **Failed:** none material.
- **Friction:** I initially missed that `anvil drift snapshot/report` already
  exist (archived DRIFT module) — the audit's first pass proposed git-archaeology
  before finding the shipped snapshot infra.
- **Improvement:** When auditing a "new" capability, grep for an existing command
  surface (`commands/*.rs`) AND archived modules before designing a data source.
- **Follow-up:** Operator to accept ADR-052; then implement the auto-snapshot
  write actor + a new INSIGHTS item, then INSIGHTS-003 on the populated series.

### 2026-05-27 — opencode

- **Task:** Triage stale automated APS reconciliation PR #1970.
- **Outcome:** Closed the PR as stale/incomplete and confirmed no in-repo GitHub
  Actions generator was present.
- **Worked:** Checking mergeability, diff scope, current `origin/main`, and
  automation references made the close-vs-fix call clear.
- **Failed:** Nothing substantive.
- **Friction:** External PR generators can create APS bookkeeping PRs that bypass
  the repo's managed `aps:index` two-surface update path.
- **Improvement:** Treat generated APS reconciliation PRs as suspect unless they
  update both module headers and `plans/index.aps.md` via the managed script.
- **Follow-up:** Disable or retire the external Codesmith/Claude Code job if it
  still exists outside the repo.
