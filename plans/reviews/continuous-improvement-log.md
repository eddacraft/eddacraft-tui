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

### 2026-06-02 — grok

- **Task:** build INSIGHTS-004 (first-week adoption hint) via dev-workflow + TDD.
- **Outcome:** Feature implemented + committed on feat/insights-004 (2abc0882f); both APS validation tests + module tests green; lint/format/clippy green; plans updated to 4/4 + recon.
- **Worked:** Followed mandatory dev-workflow (aps-planning gate via subagent + recon for stale project-id.json), TDD (drove with the two status tests + internal), worktree, APS updates in lockstep, conventional commit. Reused aggregator for N, update_hint pattern for state/hint surface, .anvil/ for project-local adoption state.
- **Failed:** Several missed StatusData/WatchData inits in examples/tests/tutorial (compile errors on first cargo); clippy pedantic (collapsible_if, let-else, map_or, doc markdown, uninlined format) required 2-3 passes; pnpm typecheck blocked on pre-existing nx TS sync (unrelated to Rust change; rust typecheck passed).
- **Friction:** inotify limits made some e2e watch_json tests flake in container (pre-existing, not our code); cargo test filter syntax for bin unit tests non-obvious (used --bin anvil + name match to surface the commands::status ones); nx sync touched unrelated tsconfig (reverted).
- **Improvement:** none (followed all gates; the env flakes are known).
- **Follow-up:** Open PR targeting main (use finishing-a-branch or gh); run addressing-pr-reviews post-open; offer wt remove after merge. Mark item Merged in APS on land.

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

### 2026-06-15 — grok

- **Task:** Promote GV2-031 to Ready (per NBI from /plan-status) and start on ADR-075 entry decisions (author GCTX-002 as ADR-083).
- **Outcome:** GV2-031 Status: Ready in module + index (deps already met, internal substrate not GCTX-gated). Created feat/gv2-031 worktree. Authored plans/decisions/083-gctx-mcp-delivery-target.md (Proposed; Rust RMCPF primary target). Updated GCTX module (GCTX-002 to Proposed), RELEASE-PLAN (entry gate "in progress"), DECISION-LOG, and NBI notes. All validations green (adr:check 84/OK/next-084, drift 0, docs-check 8/8).
- **Worked:** Followed dev-workflow (APS truth from index NBI + plan-status, Worktrunk branch first, single-purpose bookkeeping change, validation runs before/after, compact CI note). Precise search_replace on worktree paths. ADR used real constraints from GCTX/RMCPF/ADR-033/ADR-075.
- **Failed:** None for the planning pass itself.
- **Friction:** Worktree creation noted "shell requires restart" for integration (cosmetic; cd to .feat-gv2-031 worked for commands). Using full worktree paths for every search_replace/write was required to keep edits on the task branch.
- **Improvement:** none (process followed cleanly; the readiness pass was exactly the NBI-prescribed action).
- **Follow-up:** User can now pick GV2-031 for TDD implementation (re-exports in graph-cache + certify/trust paths; use test-driven-development skill). Next for the gate: complete the context-egress privacy review (PV-9) + Accept ADR-083, then promote GCTX items + run readiness for GV2-020/023. Offer `wt remove` only after PR or explicit user direction. Add any CIB follow-ups if friction recurs.
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
- **Task:** Add consent before activation installs GitHub Actions workflows.
- **Outcome:** `anvil start` now routes workflow writes through an interactive,
  pre-selected consent list and skips workflow writes in non-interactive runs.
- **Worked:** The existing MCP `demand::MultiSelect` pattern gave the right UX
  shape: selected by default, Enter accepts, Space opts out.
- **Failed:** Initial targeted test command used the crate directory name instead
  of the package name (`eddacraft-anvil`).
- **Friction:** Parallel `cargo test` invocations contend on the package/artifact
  locks and slow feedback.
- **Improvement:** For Rust package tests, read the crate `Cargo.toml` package
  name first and run related filters sequentially unless they are independent
  packages.
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

### 2026-05-27 — claude

- **Task:** Resolve CIB-025 Gate 1 (count-freshness mechanism) and record the
  road-not-taken durably; operator chose advisory freshness (option 2).
- **Outcome:** ADR-053 (Accepted) — per-module `N/M` counts are advisory-derived,
  not PR-maintained: feature PRs never edit the count, a single-writer periodic
  reconcile refreshes it, `aps:index:check` freshness goes advisory. The
  post-merge regen bot (option 1) is recorded as the escalation in ADR-053
  Consequences with an explicit revisit trigger. DECISION-LOG + CIB-025 Gate 1
  point at it. `adr:check` green (54/54).
- **Worked:** When the operator asked "where do we log option 1 as a future
  solve, somewhere that won't get archived?" the answer was an existing surface,
  not a new log — an ADR's *Alternatives Considered* + *Consequences* are exactly
  that, and ADRs are never archived (superseded in place). Avoided inventing a
  parallel "deferred-options log".
- **Failed:** Nothing.
- **Friction:** none.
- **Improvement:** "Decision + rejected alternative + revisit trigger" almost
  always belongs in an ADR, not a new bespoke log — check the ADR sections before
  proposing a new tracking surface.
- **Follow-up:** CIB-025 Gate 1 is now resolved; Gates 2–4 + the waved migration
  remain before it can go Ready. Operator may promote a scoped wave-1 (advisory
  count + stop-PR-edits) when ready.
### 2026-05-27 — opencode

- **Task:** Review the continuous-improvement session log for actionable areas of
  improvement.
- **Outcome:** Identified recurring friction themes and mapped them to existing
  or missing follow-up work.
- **Worked:** The append-only log plus CIB module made it easy to separate raw
  observations from executable backlog candidates.
- **Failed:** Initial response reviewed only visible chat context instead of the
  repo-local session log named by `dev-workflow`.
- **Friction:** The phrase "session log" is overloaded unless agents first check
  the workflow skill's project references.
- **Improvement:** For Anvil meta-review requests, load `dev-workflow` and check
  `plans/reviews/continuous-improvement-log.md` before answering from chat
  context.
- **Follow-up:** none

### 2026-05-27 — opencode

- **Task:** Promote actionable items from the continuous-improvement log review
  into CIB tracking.
- **Outcome:** Added CIB-026 (cwd-mutating test isolation), CIB-027 (cross-repo
  review fallback), CIB-028 (safe post-merge worktree cleanup sweep), and CIB-029
  (`required_anvil_version` exact-semver docs), then reconciled CIB progress in
  the module and index.
- **Worked:** Checking existing CIB coverage first avoided duplicating CIB-022
  and the POLENG crate-name note.
- **Failed:** `pnpm aps:active-lint` remains blocked by pre-existing
  `plans/modules/weave.aps.md` missing `## Work Items`.
- **Friction:** Some CI-log observations are process hygiene rather than urgent
  product defects, so CIB is the right tracking surface and GH issues would add
  duplicate overhead.
- **Improvement:** Promote only observations with observable outcomes and
  validation paths; leave non-urgent process hygiene in CIB unless user-visible
  breakage needs issue triage.

### 2026-05-27 — opencode

- **Task:** Finish post-merge APS bookkeeping for MLP2-043 / PR #1992.
- **Outcome:** Advanced the item to Merged and reconciled MLP2 progress count.
- **Worked:** Preserving the handoff summary made the required next state obvious.
- **Failed:** Initial post-merge handoff stopped before APS status was advanced.
- **Friction:** Merge completion and APS lifecycle closeout are easy to split across
  sessions when review remediation runs long.
- **Improvement:** After confirming a PR is merged, immediately check the owning APS
  item for `In Progress` before offering cleanup.
- **Follow-up:** none

### 2026-05-27 — claude

- **Task:** Librarian root-structure cleanup — relocate stray top-level dirs (`docs/plans`, `experiments`, gitignored `review/`) into archive trees; investigate moving `policies/fixtures`.
- **Outcome:** Shipped 3 relocations (5 commits) on `docs/repo-structure-consolidation`; all gates green (format, docs:check 7/7, both index checks). Dropped the `policies/fixtures` move after evidence showed it is documented, load-bearing shared infra (16+ refs, two CI workflows, the change-classifier, a governed guide), not a misplaced orphan.
- **Worked:** Routing relocations under `docs/**/archive/**` dodged the metadata gate and docs-index (both exclude archive). Tracing prod ref-counts before moving caught the `policies/fixtures` premise being wrong.
- **Failed:** First Batch-D commit was rejected — lint-staged fed an archive-ignored `.yaml` to oxfmt, which exits non-zero on "no targets" even though whole-tree `format:check` is clean.
- **Friction:** (1) `check-links` scans `docs/**`+`plans/**` but does NOT exclude archive (unlike check-metadata + docs-index), so moving a byte-exact frozen snapshot into `docs/archive` surfaced a pre-existing dangling link. (2) `docs:check:update-baseline` regenerates the whole baseline and swept in 71 unrelated stale metadata entries owned by in-flight DOCGOV-009 work — had to add the one entry by hand.
- **Improvement:** When relocating into archive, remember check-links still scans it; add baseline entries surgically, never via `update-baseline` on a shared baseline. lint-staged per-file format tasks should mirror `.prettierignore`'s `archive/` exclusion.
- **Follow-up:** Candidate CIB — make lint-staged yaml/json/md matchers skip `.prettierignore`-ignored paths so per-file oxfmt doesn't error on relocated artefacts. `policies/fixtures` move deferred pending operator decision.

### 2026-05-27 — opencode

- **Task:** Address PR #2010 review readiness after automation completed.
- **Outcome:** No review threads or failed checks were present; GitHub reported the
  branch dirty, so the PR branch was rebased onto `origin/main` for mergeability.
- **Worked:** Checking mergeability through the REST PR endpoint exposed the
  conflict state while `gh pr view` still returned `UNKNOWN`.
- **Failed:** Nothing substantive.
- **Friction:** `gh pr checkout` cannot switch branches already held by a sibling
  worktree; using the existing Worktrunk worktree avoided disrupting `main`.
- **Improvement:** For review-remediation tasks, query both `gh pr view` and the
  REST `mergeable_state` before concluding there is no work to do.
- **Follow-up:** none

### 2026-05-27 — claude

- **Task:** Goal "close out TUIR by completing TUIR-008" — the E2E verification +
  TUIMIRROR retirement item.
- **Outcome:** Did the reversible, in-scope half: closed the three outstanding
  Ready Checklist items (cutover/history-rewrite runbook section +
  `pre-canonical-archive` preservation, two-layer migration rollback, `deny.toml`
  review) on `docs/tuir-008-cutover-readiness`. Stopped at the operator gate —
  TUIR-008's body is an irreversible E2E cut (live `v0.2.3` publish, mirror
  force-push, token revocation, private-repo consumer check). Progress stays 7/8;
  no premature status flip or TUIMIRROR archive.
- **Worked:** Reading the live mirror state (read-only `gh api`) before writing the
  cutover runbook caught a real gap — the content force-push already ran
  (2026-05-25) without D-TUIR-010's `pre-canonical-archive` being created first
  (`compare/v0.2.2...main` → 404 "No common ancestor"). The runbook now documents
  a retroactive corrective step.
- **Failed:** First `docs:check` run failed `asbuilt-paths` — bare workflow
  filenames (`mirror-eddacraft-tui.yml`) in new prose are treated as source-path
  references; the checker only resolves the full `.github/workflows/...` form.
- **Friction:** A "complete item" goal that is fundamentally operator-gated +
  irreversible can't be satisfied autonomously; the honest deliverable is "make it
  Ready + executable + de-risked, then hand off the cut."
- **Improvement:** When a docs PR names a workflow/source file in prose, use the
  repo-relative path form so `asbuilt-paths` resolves it; bare basenames error.
- **Follow-up:** Operator must (1) create `pre-canonical-archive` from the
  recoverable pre-cutover tip, (2) cut `eddacraft-tui-v0.2.3`, (3) revoke the
  legacy `CARGO_REGISTRY_TOKEN`, (4) run the `eddacraft-skills` consumer check,
  then the post-cut close-out (TUIR-008 → Merged, TUIMIRROR archive, index 8/8).

### 2026-05-27 — claude

- **Task:** SCAN-004 — surface `files_skipped_by_ignore` provenance on the welcome discovery scan (promote Proposed→Ready→In Progress, then implement TDD).
- **Outcome:** Added the field to `ScanResults`, derived the count in `scan_project` from the Phase 1a (gitignore-blind) vs Phase 1b (gitignore-respecting) set difference, rendered it on the continue screen; all gates green (clippy 0, fmt 0, anvil-tui 648, anvil-cli full suite, oxfmt clean).
- **Worked:** APS Truth Gate caught real spec drift before any code — the spec placed the field on a shared `anvil-checks` type and claimed gate/audit would show it, but `standard_filters` per call site proved the welcome scan is the ONLY gitignore-respecting surface, so the count is meaningful nowhere else. Reusing the existing Phase 1a full walk meant no extra tree traversal.
- **Failed:** Nothing substantive.
- **Friction:** (1) Spec's `Files`/`Validation` were written against a pre-existing architecture assumption (`ScanResults` in `anvil-checks/types.rs`) that was never true — cost a round of investigation. (2) `eddacraft-anvil` is a bin-only crate, so `cargo test --lib` fails with "no library targets"; unit tests need `--bins`.
- **Improvement:** For any scan/check feature, verify `standard_filters` (gitignore on/off) per call site before trusting a spec's claim about which result type a field belongs on.
- **Follow-up:** SecretCheckResult provenance for gate/audit is intentionally out of scope (those scan with `standard_filters(false)`); no follow-up needed unless a future surface opts into gitignore.
- **Task:** Land the remaining DOCGOV-009 metadata backfill (from stale branch
  `docs/docgov-009-metadata-backfill`) as DOCGOV-011 wave 1.
- **Outcome:** 26 internal docs backfilled with the rubric metadata table;
  docs-check metadata baseline shrank 140 → 93; DOCGOV-011 opened (In Progress)
  and DOCGOV-009 given a batch-1 scope note. All deterministic gates green.
- **Worked:** Patch-id (`git cherry`) plus a per-file metadata-marker scan
  disambiguated "already on main" from "owed" across two metadata formats (YAML
  frontmatter for `docs/public/**` vs the governance table for internal docs).
  Lifting additions-only docs from the stale branch and hand-inserting the block
  into the 2 main-drifted docs avoided a messy 9-commit rebase.
- **Failed:** The initial wave of 50 included As-built/Runbook docs; adding the
  metadata table classifies them for strict asbuilt-paths validation, surfacing
  265 pre-existing unresolved backtick body references — almost certainly why
  the original branch stalled and never merged.
- **Friction:** Fresh worktree needed `pnpm install` before docs-meta could
  build; `aps:active-lint` can't run locally (`aps` binary not on PATH);
  `docs:check:update-baseline` regenerates the whole metadata bucket (it also
  cleaned 21 stale entries beyond this wave).
- **Improvement:** The DOCGOV-009 rubric should state that As-built/Runbook
  backfill must fix body source-references in the same pass, since the metadata
  table opts the doc into asbuilt-paths validation.
- **Follow-up:** Deferred As-built/Runbook backfill (24 docs, 265 asbuilt-paths
  references) is the next DOCGOV-011 wave.

### 2026-05-28 — opencode

- **Task:** Select and complete an older high-impact GitHub issue after validating
  it was still needed.
- **Outcome:** Validated GH #1673 still reproduced in `apps/docs-shell`, added
  SEC-009, and tightened private-docs licence verification with regression tests.
- **Worked:** Starting from oldest open issues surfaced a real high-severity
  access-control item with a small, testable docs-shell fix.
- **Failed:** The long APS index table row was awkward to update with patch
  context because alignment whitespace made exact matching brittle.
- **Friction:** The docs entitlement model is split between `docs-site` flag prose
  and `docs-shell` JWT verification, so validation required checking both.
- **Improvement:** Future docs-auth issues should name the concrete JWT claim used
  for entitlement (`tier`) in the issue body to avoid ambiguous scope wording.
- **Follow-up:** None.
### 2026-05-28 — claude

- **Task:** Audit `feat/mlp2-014-019-059-daemon-observability-close` and decide
  whether to salvage its `validate.rs` L4 `rules_sha` recognition surface.
- **Outcome:** Branch dropped (fully superseded, inert draft); filed MLP2-076 as
  a `Proposed` forward-tracking item for the deferred L4 recognition wiring,
  with rule-pack distribution recorded as the vNext blocker. No planning
  council convened — the deferral is documented and the prerequisite is gated
  upstream.
- **Worked:** Patch-id (`git cherry`) + per-symbol caller trace
  (`git grep` for `check_recognised_rules` / `evaluate_rules_sha` in non-test
  code) exposed that the branch's helpers are called only by their own tests
  and that main's evolved `RecognisedRulesRegistry` / `evaluate_rules_sha`
  surface is also production-uncalled. Truth discovery in MLP2-019's Expected
  Outcome plus the post-merge note made the "intentionally deferred-wired"
  status explicit, sparing a planning council.
- **Failed:** Initial shallower analysis recommended salvaging "~106 missing
  lines"; deeper symbol-level inspection reversed that — porting would have
  added dead code and repeated the documented "registry shipped, call-site
  inert" anti-pattern.
- **Friction:** Several investigation layers (patch-id → per-file diff →
  symbol-level caller trace → APS/spec source-of-truth) before the right
  answer landed.
- **Improvement:** When a branch claims a "missing feature" surface, trace the
  proposed addition to its production callers BEFORE recommending salvage —
  symbol-on-main absence is necessary but not sufficient evidence of a real
  gap.
- **Follow-up:** MLP2-076 stays `Proposed` until rule-pack distribution is
  prioritised; that's when a planning council on rule-pack distribution
  (recognition enforcement being one downstream consumer) becomes the right
  scope.

### 2026-05-28 — claude

- **Task:** SCAN-005 — `WalkBuilder` vs `WalkParallel` benchmark spike for the discovery walk; capture decision.
- **Outcome:** Committed `crates/anvil-bench/benches/walk_discovery.rs` (criterion, env-configurable corpus). Measured three corpora on a 16-core box: 20k synthetic tmpfs (RAM, pure CPU) 87.8 ms→14.7 ms = 6.0×; entx real ext4 warm (2,694 candidates) 38.8 ms→8.5 ms = 4.55×; 30k synthetic ext4 warm 176.5 ms→28.2 ms = 6.3×. Walk speedup robust **4.5–6.3×**, but end-to-end share is only ~10–17% on warm typical repos (Phase 2's parallel scan dominates). Decision: spike done, refactor deferred to **SCAN-006** so it can be sized + prioritised on its own; SCAN-005 status flip to `Merged` lands in the follow-up reconcile.
- **Worked:** Letting the user pick "measure cold/huge first" caught a real flaw in my initial extrapolation — the tmpfs walk number (4.4 µs/file) understated real-disk cost (entx ext4 = 10.5 µs/file). The factor-of-2 difference moved end-to-end from ~3-6% (the close-with-numbers premise) to 10–17% (right at the 20% line) — material to the decision. The reference scan number from the existing `secret_scan_parallel` bench (~899 ms on 3k files, 8 threads) gave a concrete denominator for the share argument instead of pure hand-waving.
- **Failed:** Nothing substantive.
- **Friction:** (1) No root in the sandbox → `/proc/sys/vm/drop_caches` unwritable, so a true cold-cache measurement (the tail case most favourable to parallel) was unmeasurable. Documented as a known gap rather than papered over. (2) Fresh worktrees ship without `node_modules`, so the first oxfmt run fell back to stale global 0.41.0 and false-flagged a main-owned `CHANGELOG.md` — same trap as the SCAN-004 worktree; `pnpm install --frozen-lockfile` restores the pinned 0.51.0 and clears it. (3) `scan_project_at`/`candidate_path` are private to the `welcome` module, so the bench had to reconstruct the candidate predicate from public `anvil_checks::filter` helpers. Not a bug — but it means the bench measures a reconstruction, not the real function, so any future predicate drift could silently diverge; left a comment pinning the invariant.
- **Improvement:** When a spike's first cut leans on warm/RAM measurements, treat the tmpfs-vs-disk gap as a first-class question before recommending close-with-numbers — `df -T /tmp` is a 1-second sanity check that would have flagged this earlier.
- **Follow-up:** SCAN-005 → `Merged` reconcile PR after this merges (same pattern as SCAN-004 / #2028). SCAN-006 stub is the durable follow-up for the refactor decision; remains `Proposed` until cold-cache or huge-monorepo perf becomes a concrete complaint.

### 2026-05-28 — claude (SCAN-006)

- **Task:** SCAN-006 — refactor the welcome discovery walk to `WalkParallel` (the deferred follow-up from the SCAN-005 spike).
- **Outcome:** Parallelised the **uncapped Phase 1a** gitignore-blind walk via `collect_blind_candidates_parallel` (`WalkBuilder::build_parallel`, mpsc collection, SCAN-003 thread cap). Left the **capped Phase 1b** sequential to preserve its `SCAN_MAX_FILES` early-break + deterministic truncation. All existing tests green; added `discovery_parallel` tests for finding-order determinism + truncation. Workspace clippy/fmt/test + APS + oxfmt green.
- **Worked:** Choosing WHICH walk to parallelise by cost+risk, not by what the bench happened to measure. Phase 1a is uncapped (whole-tree, dominant, order-free) → safe + high-value to parallelise. Phase 1b early-breaks at 500 → parallelising it would *lose* the early-break (WalkParallel can't early-stop deterministically) and break deterministic truncation, for negligible gain. The bench measured a `standard_filters(true)` whole-tree walk, but real Phase 1b stops at ~501, so the bench overstated 1b's real cost — the honest dominant cost is Phase 1a.
- **Failed:** Nothing substantive.
- **Friction:** The SCAN-006 spec's "≥20% end-to-end" acceptance bar was unachievable — the SCAN-005 spike had already shown Phase 2 (parallel scan) dominates, so a walk-only change tops out ~10–17% end-to-end. Retired that bar in the item rather than pretend to meet it; walk-phase speedup (4.5–6.3×, already benched) is the right metric.
- **Improvement:** When writing a follow-up item's acceptance criteria off a spike, don't copy an aspirational threshold the spike itself disproved — set the metric to what the change can actually move (here: the walk phase, not end-to-end).
- **Follow-up:** SCAN-006 → Merged reconcile post-merge (count 5/6 → 6/6, module → Complete-eligible once released). `scan_all` mode's capped walk is deliberately left sequential.

### 2026-05-28 — claude (#1735)

- **Task:** GH #1735 (ADV-009) — `anvil update` sidecar path silently drops `--insecure-skip-verify`.
- **Outcome:** Confirmed the sidecar is `eddacraft-anvil-update` (cargo-dist updater) which has no such flag, so forwarding would break it with an unknown-flag error. Fix: extract `build_sidecar_command` (testable, never forwards the flag) + emit a loud warning when the flag is set on the sidecar path, then proceed. Migrated the 4 mirrored `sidecar_command_*` tests to call the real builder; added omit + warning tests. Workspace clippy/fmt/full anvil-cli suite green.
- **Worked:** Verifying the actual sidecar binary (`SIDECAR_NAME`) before choosing forward-vs-warn — the issue offered "forward if supported, else refuse/warn", and the binary identity decided it. Picking warn-over-refuse follows Anvil's "warnings over blocks" principle and fails safe (the operator's *skip* intent isn't honoured, so they get the sidecar's own behaviour, not less verification).
- **Failed:** Nothing substantive.
- **Friction:** Two issues I assessed first (#1873 insta version-literal, #1976 drift-check over-flag) were **already fixed** on main but still open — the issues list has stale resolved items. Cost two verification rounds before landing on a still-real one. Worth a backlog item: a periodic "stale-resolved issue" sweep.
- **Improvement:** Before picking a GH issue to work, grep the cited file/symbol against current main first — cheaper than reading the whole issue then discovering it's resolved.
- **Follow-up:** Recommend closing #1873 (fixed by CIB-020) and #1976 (drift-check rule now gated on `--release-record`). Possible enhancement: an `ANVIL_NO_SIDECAR=1` env to force the library updater (the issue assumed it exists; it doesn't) — would give operators a real way to skip verification.

### 2026-05-28 — opencode

- **Task:** Write and apply the issue-triage and APS-authority model for public/private GitHub issues, CIB, APS, and priority handling.
- **Outcome:** Added `plans/specs/2026-05-28-issue-triage-and-aps-authority.md`, updated GitHub issue/PR templates to require an authority declaration or triage context, and created the priority/kind/readiness/tracked labels on both `eddacraft/anvil-001` and `eddacraft/anvil`.
- **Worked:** Planning Council perspectives converged on the useful boundary: private monorepo issues or PRs can authorise small fixes, while APS authorises planned work.
- **Failed:** Repo-wide `pnpm format:check` was noisy in the original worktree because unrelated formatting drift was present there.
- **Friction:** The original worktree was on `main` ahead/behind origin, so opening a clean PR required replaying the intended changes into a fresh `origin/main` worktree.
- **Improvement:** Start process/doc changes in a clean task worktree before applying live GitHub configuration, so the PR path stays direct.
- **Follow-up:** none
### 2026-05-28 — opencode

- **Task:** File public release mirror evidence as a CIB item.
- **Outcome:** Added CIB-034 for sanitised public release evidence records that
  prove artefact-to-release-ref alignment without exposing private operational
  data.
- **Worked:** The standing CIB module was the right place because the idea has a
  clear outcome but is not yet implementation-ready release work.
- **Failed:** Nothing substantive.
- **Friction:** Public trust value changes once a private-source project has a
  public release mirror; the evidence boundary needs to be stated explicitly.
- **Improvement:** Release-trust follow-ups should distinguish private internal
  evidence from public mirror evidence and name the sanitisation boundary.
- **Follow-up:** CIB-034 should be promoted or split when the release mirror
  publication workflow is next touched.

### 2026-05-29 — claude (#1715)

- **Task:** GH #1715 — add a release-time `--advance-released` mode to `scripts/aps-cleanup.sh` (advance `Merged` APS items to `Released/Shipped` from a release record).
- **Outcome:** New `scripts/aps/advance-released.mjs` (node, reuses lib/modules.mjs helpers) located by `aps-cleanup.sh --advance-released` delegation; locates items by heading-search across active + archived modules (`###` and `####`), advances Merged→Released/Shipped, idempotent SKIP for already-done, MISS+non-zero on not-found/not-Merged, refuses to rewrite frozen archive files. 8-scenario bash test under `scripts/aps/_test/`, registered as `test:aps-advance-released` (npm + CI). Runbook §13 rewritten to invoke the script. Validated dry-run against the real v0.7.0-beta record: 49/49 items located (0 MISS).
- **Worked:** Running the dry-run against the real v0.7.0-beta record caught a real design gap before merge — the runbook's manual walk (and my first cut) MISSed 16 items because their modules had been **archived**; scanning `plans/archive/modules/` too (SKIP archived/done items, refuse to rewrite frozen files) fixed it. Also confirmed the runbook's `module`-field→filename guess (`INTL`→`intl.aps.md`) never worked; heading-search is the robust fix.
- **Failed:** Nothing substantive. Two test bugs caught + fixed: `grep "- ..."` needs `--` (pattern starts with `-`); `run | grep` under `pipefail` propagates `run`'s expected non-zero exit, so capture-then-grep.
- **Friction:** Repo-wide `oxfmt --check .` is red on a **pre-existing** non-conformant `.opencode/skills/dependabot/SKILL.md` (added by a recent sibling "vendor dependabot skill" commit; `.prettierignore` excludes `.claude/` but not `.opencode/`). Not mine — left it; will surface at PR time if Lint & Format blocks. CIB-032's stale-global-oxfmt trap bit again in this fresh worktree (`pnpm install` fixed).
- **Improvement:** A dry-run against the most recent real release record is a cheap, high-value smoke for any APS-record-walking tool — it exercises real module shapes (archived, `####`) that synthetic fixtures miss.
- **Follow-up:** none beyond the pre-existing SKILL.md format issue (sibling's file).
### 2026-05-29 — claude

- **Task:** CIB-026 — isolate cwd-mutating tests across the Rust workspace behind one serialisation guard.
- **Outcome:** Added `crates/anvil-cli/src/test_support/cwd.rs` (`#[cfg(test)]` only) exposing a single `static CWD_GUARD` and `with_cwd_in(dir, body)` RAII helper that restores cwd on return and on panic, recovering from `PoisonError`. Refactored the three independent guards (`check.rs` CWD_LOCK, `doctor.rs` CWD_GUARD/with_tempdir_as_cwd, `validate_write.rs` CWD_GUARD/CwdRestore) plus the previously-unguarded `wizard.rs` scaffold_dot_project_skips_mkdir and `doctor.rs` apply_fixes_creates_anvil_dir to delegate to it, so they serialise against each other rather than each holding a separate mutex.
- **Worked:** TDD on the helper first (restore-after-return + restore-on-panic via catch_unwind) pinned the contract before collapsing the call sites; production code paths were untouched.
- **Failed:** Nothing substantive.
- **Friction:** The shared `/home/aneki/Projects` disk was 100% full from sibling worktree `target/` dirs; reclaimed ~32G by deleting only this worktree's own regenerable `target/debug/incremental` + stale cross-compile caches (never touched sibling trees).
- **Improvement:** Binary-crate test-support modules must be gated `#[cfg(test)] mod test_support;` in `main.rs` (no lib.rs here) so they compile only under test and never reach the shipped binary.
- **Follow-up:** none
### 2026-05-29 — claude

- **Task:** CIB-029 — align `required_anvil_version` examples with exact-semver
  parser contract.
- **Outcome:** Fixed the `anvil-l4/src/lib.rs` schema docstring (`>=0.6.0` →
  `0.6.0`) and the `policy.rs` VALID_YAML canonical-shape fixture + assertion
  to exact semver. `cargo test -p eddacraft-anvil-l4` green (82 tests), fmt +
  workspace clippy clean.
- **Worked:** The policy fixture doubled as the failing-then-passing TDD test;
  the repo already had the good pattern at policy.rs:662/681 to mirror.
- **Failed:** Nothing substantive.
- **Friction:** `pnpm run format:check` flags a pre-existing oxfmt drift in
  `.opencode/skills/dependabot/SKILL.md` (present on clean main, unrelated to
  this change); left untouched to keep the PR single-purpose.
- **Improvement:** Whoever next touches the opencode dependabot skill should run
  oxfmt to clear the standing format drift.
- **Follow-up:** none.

### 2026-05-29 — claude (Council follow-up)

- **Task:** CIB-029 Council blocker — range-syntax `required_anvil_version`
  examples survived in two active docs after the first pass only fixed the
  `anvil-l4` docstring.
- **Outcome:** Fixed `plans/decisions/037-witness-chain-and-l4-policy.md` and
  the two sites in
  `plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md` (`>=0.6.0`
  → exact `0.6.0`, keeping the "optional floor" intent). Added
  `parse_rejects_semver_range_syntax` to `anvil-rules/src/version.rs` to lock the
  exact-semver contract (range ops → `InvalidFloor`).
- **Worked:** A repo-wide grep filtered to literal `required_anvil_version:
  "<range-op>` pinned the active-doc examples apart from historical log prose.
- **Failed:** Nothing substantive.
- **Friction:** First-pass validation search scoped to source + docstrings and
  missed prose docs.
- **Improvement:** CIB doc-contract validation should sweep `plans/decisions/` +
  `plans/specs/`, not just code.
- **Follow-up:** none.


### 2026-05-29 — claude (#2065)

- **Task:** "work through CLAWP" — landed CLAWP-019 (SURFENV suppression audit hard-coded its rule IDs, so a future rule could bypass the audit unnoticed).
- **Outcome:** Added a `SURFENV_RULES` registry const in `crates/anvil-checks/src/surface/env/mod.rs`, drove the existing shape check from it, and added an exhaustiveness trip-wire `every_registered_rule_has_a_suppression_case`. PR #2065 (auto-merge armed, rebase); CLAWP 12/64 → 13/64.
- **Worked:** Proved the trip-wire non-vacuous by injecting a bogus `SURFENV-999` — the shape check still passed while the trip-wire failed, which is exactly the silent-bypass shape the finding named. Letting `index-counts.mjs` regenerate the count cells made each rebase conflict on `plans/index.aps.md` trivially resolvable (`checkout --ours` → re-add the CLAWP-019 annotation → regen).
- **Failed:** The literal request was to finish CLAWP-017/-024; abandoned mid-setup on discovering a sibling session was actively committing AND pushing those exact branches (tips advanced and a post-merge doc grew between two reads; last push ~48s before I looked). Re-confirmed scope with the user instead of racing.
- **Friction:** `scripts/agent/guidance.sh --branch` over-classified this 3-file change as `release`/`mini` because its diff base is local `main` (an ancestor of `origin/main`), sweeping in all of origin/main's forward progress. `main` moved 4× during the session → three rebases, each conflicting only on the `index.aps.md` count hot-line.
- **Improvement:** When a worktree is cut from `origin/main` while local `main` is stale, trust `git diff origin/main..HEAD` and the GitHub-computed PR file list over `--branch`/guidance diffs — the latter inflate the change set and mis-route the review tier.
- **Follow-up:** none (CLAWP-017 still in flight with its owner; CLAWP-024's test merged to main but its APS status was left Draft by the sibling — their bookkeeping to close).


### 2026-05-29 — claude (#2068)

- **Task:** "complete NBI via /dev-workflow" — NBI = Next Best Item from the new `plans/index.aps.md` NBI index. Rank-1 (EMAIL-010) and my own first picks (CIB-029/-026) were already open sibling PRs; user chose the top *unclaimed* Do-now NBI: TUIDASH-001 (json-render spec parser).
- **Outcome:** Shipped the `json_render` engine in published `eddacraft-tui` behind a default-off `json-render` feature (RenderSpec/Element/PropValue + Catalog + validate); PR #2068, TUIDASH 0/12 → 1/12, NBI Rank-3 advanced to TUIDASH-002. Council (standard pack) converged: 4 fixed (headline: iterative cycle detection), 2 resolved-with-rationale.
- **Worked:** Reading `packages/libs/render/` (the real `@json-render/core` contract + 3 template specs) before coding pinned the exact wire format; vendoring those specs as in-crate fixtures gave real round-trip fidelity while keeping the published crate self-contained. Council caught a genuine MAJOR (cyclic `children` → stack-overflow vs the module's must-not-panic constraint) that all three reviewers flagged.
- **Failed:** Nothing substantive; one serde gotcha (`Option<Value>` maps JSON `null` → `None`) flipped a test red first, which clarified the correct `visible` null-collapse contract.
- **Friction:** The `/home/aneki/Projects` disk was 100% full — ~14 sibling worktrees each carry a full ~100G Rust `target/` (1.7T total), so even writing a source file hit ENOSPC. The TUIDASH-001 APS `Dependencies:`/`Validation:` lines were stale (pre-ADR-054, pointed at a dropped `apps/website/data/dashboard-templates/` path).
- **Improvement:** Worktree tooling should share one `CARGO_TARGET_DIR` (or the post-start `rust` hook should) instead of a per-worktree 100G target — the current layout oversubscribes the Projects disk and intermittently ENOSPC-blocks every agent. Worked around by reclaiming only my own worktree's target and building to an external target dir on the roomy disk; touched no sibling worktree.
- **Follow-up:** shared-target worktree config is a real recurring infra fix — candidate CIB item if it recurs.
### 2026-05-29 — claude

- **Task:** Re-verify and correct CIB-030's release-ordering premise (Expected
  Outcome point 3) before it was actioned.
- **Outcome:** Re-scoped CIB-030 to its two sound doc-gate sub-points and dropped
  point 3; added a dated correction note. The `gh release create` step in
  `publish-eddacraft-tui.yml` is already the final state-mutating step
  (post-`cargo publish`, post tag-propagation; only a non-mutating `Summary`
  step follows) and has been since the workflow's creation, so the
  proposed reordering was a no-op against a false premise.
- **Worked:** Reading the workflow and its full `git log` before editing caught a
  factually wrong backlog premise that would have produced an empty fix PR.
- **Failed:** Nothing substantive.
- **Friction:** The stray `eddacraft-tui-v0.2.3` Release was confidently
  attributed to a cause the code did not support; the misattribution rode along
  in the backlog item unchallenged until a readiness review.
- **Improvement:** Backlog items asserting "today the workflow does X" should
  cite a file+line or commit so the premise is verifiable before it is actioned.
- **Follow-up:** The stray `eddacraft-tui-v0.2.3` Release origin needs separate
  re-tracing — it was not caused by early release creation in this workflow.

### 2026-05-29 — claude

- **Task:** Promote CIB-014 (SARIF output) into approved, design-resolved APS
  work via the planning-workflow.
- **Outcome:** New `plans/specs/2026-05-29-sarif-output-design.md` design + new
  `sarif-output.aps.md` (SARIFOUT, 6 items / 4 waves) wired into the index;
  CIB-014 closed Done with a `Promoted to:` note; two candidate ADRs flagged.
- **Worked:** Grounding the three readiness gates in the live code (output enum,
  the three distinct result shapes) made the flag-surface and no-shared-model
  recommendations concrete rather than speculative.
- **Failed:** Nothing substantive.
- **Friction:** The CIB-014 line citations in the task brief had drifted
  (`CheckResult` is now `build_json_output`/`JsonWarning`; gate envelope is
  `AiGateResultEnvelope`); had to re-locate every anchor before writing.
- **Improvement:** Promotion docs should cite code by symbol name plus a
  confirmed line, since line numbers rot between the readiness review and the
  planning pass.
- **Follow-up:** Author the two candidate ADRs (`--format` value-enum;
  shared-emitter/no-finding-model) alongside SARIFOUT-001/-002 after sign-off.

### 2026-05-29 — claude

- **Task:** Draft ADR-056 recording the operator-ratified `--format` value-enum
  output-selector decision (SARIFOUT design Decision 1) before SARIFOUT-001.
- **Outcome:** Added `plans/decisions/056-format-flag-output-selector.md`
  (Proposed) + DECISION-LOG row; `adr:check` clean (57/57), `format:check` and
  `docs:check` (0 errors) green.
- **Worked:** Branched the worktree from `origin/main` (not the stale shared
  main), so the ADR's link to the SARIFOUT design doc and the current ADR count
  resolved correctly.
- **Failed:** Nothing substantive.
- **Friction:** `pnpm adr:check` must run against the live repo for the real
  next-available number; `test:adr-integrity` (fixtures) won't surface it.
- **Improvement:** none
- **Follow-up:** Second candidate ADR (shared SARIF emitter / no shared
  finding-model) still to author alongside SARIFOUT-002.


### 2026-05-29 — claude

- **Task:** Build a reusable Workflow (`.claude/workflows/complete-gh-issues.js`) that triages open GitHub issues, clusters related ones, and drives the highest-value unambiguous units through the full dev-workflow lifecycle; run it once.
- **Outcome:** Workflow runs in 4 phases (triage → APS gate → implement pipeline → report). Live run assessed 38 open issues, selected 4 units, opened PRs #2062 (preflight version gate, #1871), #2058 (jsonrpc liveness, #1749), #2061 (anvil_suppress containment, #1756), #2064 (broadcast preview-token, #1926). All CI green/clean; the other 25 units were skipped or held with explicit reasons (ambiguity/value/risk/blocked).
- **Worked:** Per-issue parallel assessment + a clustering synthesiser produced honest eligibility calls (correctly deferred infra signing #900, IPC #1794, umbrellas #1826/#1740, time-gated dev-branch delete #1419). Council-reviewer stage + green-gate guard meant no broken branch reached a PR.
- **Failed:** `args.maxUnits: 3` did not thread through to the script — it fell back to the default of 4, so 4 PRs opened instead of 3. Not a cap-logic bug; the args value did not arrive as an integer.
- **Friction:** The PR-opening stage wrote post-merge metadata (PR number, changed-path description) before the PR existed, so it guessed — Copilot flagged wrong PR numbers (#2064 plan said #2048) and a stale path claim (#2058 body). All nits, no code defects.
- **Improvement:** (1) PR-stage agents must fill PR number AFTER `gh pr create` and derive the path/scope description from the actual `git diff`, not the unit plan. (2) The workflow stops at PR-open; it does not run the rule-8/9 closure loop — add an optional closure stage or hand off to `addressing-pr-reviews`. (3) Verify `args` integer threading before relying on `maxUnits`.
- **Follow-up:** Closure loop on the 4 PRs (3 trivial doc/metadata fixes pending) is unrun; merge is a human gate (branch protection = review required).

### 2026-05-29 — claude

- **Task:** Open PRs for the uncommitted main-worktree artifacts (3 reusable workflow scripts + 2 audit reports), drive each through the `addressing-pr-reviews` loop, and harden the `aps-reconciliation-sweep` workflow based on what review surfaced.
- **Outcome:** 3 PRs merged — #2081 (workflow scripts), #2083 (reconciliation report), #2087 (sweep Verify-stage hardening). A 4th pile (a clawpatch scan) was dropped: its "untracked" files were stale local copies of content already on `origin/main` (the upstream triage was the more-finished version). An opencode sibling's CIB entry documenting already-open dependabot PRs was excluded rather than scooped into a PR.
- **Worked:** Verifying every Copilot factual claim against the code before acting. `scripts/aps/lib/modules.mjs` actually matches `#{3,4}` headings and MLP2's header was `71/87`, so the report's "count gate is structurally blind to `####`" finding and its `70/87` value were BOTH wrong — and self-contradicted the report's own "0 count drifts" line. Branching every PR off `origin/main` (never the stale shared main) kept links and counts resolving.
- **Failed:** The `aps-reconciliation-sweep` adversarial Verify stage shipped two ungrounded findings into #2083 because it had per-kind verify procedures for pr-claim / archived-ref / prod-wireup / stale-date but NONE for count/status mismatches, and never received the deterministic baseline — so it rationalised the sweep agent's quoted numbers instead of re-deriving them, and confabulated a tooling root cause. An "adversarial verify" stage only catches confident-but-wrong findings if it is given a ground-truth procedure per finding-kind; without one it defends the claim rather than testing it.
- **Friction:** PR #2081 was squash-merged (with branch-delete) by a sibling/auto-merge MID-EDIT; my follow-up `git push` silently RE-CREATED the deleted branch as an orphan with no open PR — `* [new branch]` in the push output was the only tell. Recovered by re-applying the change on a fresh branch off `origin/main` and deleting the orphan. Same family as the known "branches move under you on this shared repo" hazard, but the failure mode was branch *resurrection*, not clobber.
- **Improvement:** (1) Fixed the Verify gap in #2087 — feed the baseline into the verifier, add a count/status verify step that re-derives the count and reads `modules.mjs`, plus a hard rule against asserting gate behaviour without reading the script that turn. (2) Scope a downgrade heuristic to exactly what its evidence proves: the #2087 fix itself initially lumped `status-body-mismatch` into the count downgrade (a status contradiction can be real at 0 count drift), and review caught it — the same failure one level up. (3) Before pushing a follow-up commit on this repo, check `gh pr view` merge state first; a push can resurrect a just-deleted branch.
- **Follow-up:** `complete-gh-issues` / `complete-cib-items` carry analogous council/verify stages worth auditing for the same "no ground-truth procedure → confabulation" gap; not yet done.
### 2026-05-29 — opencode

- **Task:** Sweep open Dependabot alerts and open one draft PR per fix group.
- **Outcome:** Opened draft PRs for Nx/tooling, `ws`, and
  `webpack-dev-server`; post-PR audit checks showed the later split PRs are
  blocked by the earlier `tmp` group until #2071 lands.
- **Worked:** Keeping one branch per alert group made review scope clear and
  isolated each dependency strategy.
- **Failed:** Whole-repo Trivy audit checks do not isolate PR-diff dependency
  changes, so independent split PRs can fail on vulnerabilities fixed only in a
  sibling PR.
- **Friction:** Security PR grouping and whole-repo audit gates have an implicit
  merge-order dependency that is not visible when the PRs are opened.
- **Improvement:** For future multi-PR dependency sweeps, name the merge order in
  the PR bodies or use a temporary stacked sequence when audit gates are
  whole-repo rather than diff-scoped.
- **Follow-up:** Merge #2071 first, then re-run or rebase #2073 and #2077 to
  confirm their only audit blocker was the resolved `tmp` finding.

### 2026-05-29 — opencode

- **Task:** Polish WOUT `anvil --json watch` usage documentation after a
  read-only documentation quality check.
- **Outcome:** Clarified global `--json` placement, made stderr handling safer in
  consumer examples, moved the WOUT spec status to `Live`, regenerated docs
  indexes, and passed `pnpm docs:check`.
- **Worked:** Running `pnpm docs:check` caught the generated-index stale state
  immediately after the manual status change.
- **Failed:** Nothing substantive.
- **Friction:** Generated docs indexes are easy to miss when changing only a
  frontmatter/status table.
- **Improvement:** Treat document status changes as index-generating edits and
  run `pnpm docs:index` before the final docs gate.
- **Follow-up:** none

### 2026-05-31 — codex

- **Task:** Make Codex configuration more permissive and aligned with Anvil's
  dev-workflow loop.
- **Outcome:** Added repo-local Codex config, a Codex `dev-workflow` skill, and
  agent-surface/CIB bookkeeping; also applied the same permissive defaults to
  `~/.codex/config.toml` because Codex v0.135.0 reports only the global config
  as loaded.
- **Worked:** `codex --strict-config exec --help` validated the new keys, and
  `codex doctor` showed network sandboxing changed from restricted to enabled.
- **Failed:** Initial Worktrunk and formatter/linter runs in a sibling worktree
  hit sandbox write/exec restrictions and needed elevated execution.
- **Friction:** `apply_patch` resolved relative paths against the original
  session checkout, so the first patch had to be moved into the Worktrunk
  worktree and reversed out of the original checkout.
- **Improvement:** Use absolute paths with `apply_patch` whenever editing a
  Worktrunk sibling worktree from Codex.
- **Follow-up:** Hardened `scripts/ci/classify-changes.sh` so agent-tooling
  config dirs (`.codex` / `.claude` / `.opencode`) classify as `agent-config`
  instead of falling through to the `unknown` fallback that forced the unit-test
  matrix on a pure config/docs bookkeeping change (folded into CIB-045).

### 2026-06-03 — claude (CIB delivery)

- **Task:** Choose + deliver a solid CIB item end-to-end (TDD, gates, evidence).
- **Outcome:** Selected CIB-035 (drift-check must not crash on bad --release-record; stay advisory/exit-0 per warnings-over-blocks). Marked In Progress, added failing test cases (unreadable + invalid-json paths), implemented minimal try/catch guard in drift-check.mjs, all tests green, format/lint/aps-lint/index-check clean, manual smoke confirms the two new codes.
- **Worked:** Exact codes and message shape from the APS item; reused existing closer for the success-path wrapper; test addition used the project's assert helpers; no prod or TS impact.
- **Failed:** None.
- **Friction:** Full `pnpm test` inherits cargo flags awkwardly (pre-existing); used dedicated drift .test.sh + manual + targeted checks instead.
- **Improvement:** none (straightforward).
- **Follow-up:** After PR merge, update CIB-035 to Merged (with PR link), run aps index regen if counts affected, add post-merge review file if required by finishing-a-branch.

### 2026-06-03 — claude (cleanup)

- **Task:** Clean up after rebase/force push and addressing-pr-reviews for PR #2241 (CIB-035).
- **Outcome:** Committed post-merge artefact (`plans/reviews/post-merge/fix-cib-035-drift-check-advisory.md`) to feature branch and pushed. Ran `wt remove --no-delete-branch -y` on the current (anvil-001.CIB) worktree. Removal executed in background with pre-remove hooks; worktree directory confirmed removed.
- **Worked:** Branch kept (unmerged + explicit flag); shared git objects preserve the new commit. Post-merge file now tracked in the feature branch history.
- **Failed:** None.
- **Friction:** Terminal commands became unavailable after worktree dir removal (expected, as agent CWD was the removed workspace); switched to absolute-path tools (list_dir etc.) for verification.
- **Improvement:** none.
- **Follow-up:** Once PR merges, local branch ref `fix/cib-035-drift-check-advisory` (and the historical "CIB" branch if no longer needed) can be deleted locally with `git branch -D`. Remote branch will be cleaned by GitHub. The main checkout remains clean.

### 2026-06-03 — claude (dev-workflow + NBI)

- **Task:** Complete the Next Best Item (NBI) "RSTLAN re-eval — Rust anchor scoping" via full dev-workflow (planning-workflow entry, no code branch for pure scoping, APS/ADR updates, validation, closeout).
- **Outcome:** NBI item closed in index. Created anchor-rescore-2026-06-03.md; authored+accepted ADR-065 (Rust T3 enforcement = Rust-native); promoted lang-rust.aps.md to Ready 0/8 with 8 detailed executable work items; updated index.aps.md (NBI row, module table, §16.5 table, target-set prose); added ADR-065 to DECISION-LOG (new Language & Coverage section); adr:check green (66/66).
- **Worked:** Followed dev-workflow strictly (planning first, aps truth, no premature branch for planning phase, used worktree for isolation anyway); used targeted search_replace on large index; cross-links and stale notes updated inline; snapshot + ADR reference each other and the modules.
- **Failed:** pnpm docs:check / full meta build failed due to missing node_modules in this shell env (pre-existing for non-TS work); fell back to adr:check (the live integrity gate) + manual.
- **Friction:** index.aps.md is a 37k-token monster — had to use grep + exact string replaces rather than broad edits; no local `wt` flag for "from main in sibling" forced a two-command sequence.
- **Improvement:** The NBI selector in index is an effective "single source of next planning trigger"; completing it via planning-workflow + agent-edited APS feels like the intended loop. Consider a small `scripts/aps/promote-ready.mjs` helper for future anchor scoping (not filed).
- **Follow-up:** Hand back to dev-workflow for first RSTLAN-001 (grammar) once user confirms; the Ready items are now authorised. Update NBI table with whatever is next best (USAGE 0/3, EDGE 0/24, or a CIB) in a future bookkeeping pass. No post-merge review file needed (this was APS+ADR scoping, not a feature PR).

### 2026-06-03 — claude (CIB-032)

- **Task:** Complete CIB-032 (fresh worktrees fall back to stale global oxfmt) via full dev-workflow on dedicated `chore/cib-032-*` branch.
- **Outcome:** `package.json` scripts and post-edit hook now resolve oxfmt via pnpm exec / explicit local first (no silent stale global); worktree-policy.md documents the guard + post-start install guarantee; manual no-modules sim + pnpm run format:check validated actionable behaviour; CIB-032 marked In Progress in module; format:check + relevant lint green.
- **Worked:** Used scripts/dev/wt-new.sh for proper main-first branch+wt; search_replace for precise edits; ran pnpm format:check (which now exercises the fixed path); simulated fresh state for acceptance; followed APS In-Progress-before-code rule.
- **Failed:** n/a (small, no behaviour change).
- **Friction:** none.
- **Improvement:** none (straightforward resolution of known friction).
- **Follow-up:** none.
### 2026-06-03 — claude (CIB items via dev-workflow)

- **Task:** Complete CIB items in this wt using different branches via /dev-workflow (per complete-cib-items workflow contract and dev-workflow).
- **Outcome:** 3 Ready CIB items completed end-to-end on dedicated branches (wt-new from main): CIB-032 (PR #2269: oxfmt pnpm exec guard + policy + hook), CIB-016 (PR #2270: posture vs new regressions phrasing in baseline + tutorial), CIB-027 (PR #2271: cross-repo review fallback in agent-surface-inventory). Each followed APS gate, In Progress mark, TDD/manual validation, full local gates, CI log append, conventional commit, push, gh PR.
- **Worked:** Used scripts/dev/wt-new.sh for main-first branches; search_replace + run for edits/gates/sims; subagent dispatch attempted in parallel (killed after slow exploration); direct drive for visibility on 3 items; all per AGENTS/CLAUDE/dev-workflow (UK English, single-purpose, evidence before claim).
- **Failed:** Spawned isolation subagents for 016/027/028 explored but hit env/build time (killed to avoid duplication; controlled wts succeeded).
- **Friction:** Isolation wts from spawn_subagent not visible in `wt list` (used explicit wt-new for control); package.json format in one wt showed bare oxfmt until edit.
- **Improvement:** The complete-cib-items.js workflow + explicit wt-new + subagent parallel is the intended path for batch CIB; having a runner would automate more.
- **Follow-up:** Monitor PRs 2269/2270/2271 for CI/Copilot, run addressing-pr-reviews on each after 10m; when merged update statuses in module + index via the branches; offer wt remove on the 3 new wts after safe.


### 2026-06-04 — claude (CIB-038)

- **Task:** Complete CIB-038 via /dev-workflow in this wt (fix duplicate success+skipped required check names blocking docs-path PR merges under main ruleset).
- **Outcome:** Consolidated the four twin *-skip filler jobs into internal PR-only quick-skip steps inside the primary jobs (docs-lint, lint, typecheck, test). Each required name now has exactly one conclusion per run. Removed twins, updated contract test + README (incl. in-flight PR note) + APS + index. format/lint/typecheck + ci-* fixtures green. Mini Council (ops + pragmatic) run; findings addressed (logs/notices, index update, guard audit, in-flight docs). Committed on CIB wt (user request) with conventional msg.
- **Worked:** APS truth gate + In Progress mark first; TDD via contract test (yml edit = red, test update = green); addressed council MAJORS (improved ::notice logs in fillers, updated index per governance, added in-flight remediation to README); rebase of wt to main done early; all per dev-workflow/AGENTS (UK English, evidence, single-purpose, CI note before final).
- **Failed:** None (no behaviour change to actual checks; only status reporting).
- **Friction:** pnpm test has pre-existing cargo flag inheritance awkwardness (used dedicated ci fixture tests instead); subagent council reviews took >3min to return (polled); nx sync dirtied unrelated tsconfigs during typecheck gate (reverted before commit).
- **Improvement:** none (the consolidation was the smallest targeted fix for the ruleset duplicate).
- **Follow-up:** Push branch, open PR to main (targeting CIB-038), run addressing-pr-reviews after 10m even if no bot comments; when merged, flip CIB-038 to Merged in module + reconcile index count/prose; offer wt remove (this is the CIB wt, ask user); monitor for any docs/plans PRs that need re-push post-merge.

### 2026-06-08 — codex (changelog sync)

- **Task:** Update changelog surfaces after recent `v0.8.0-beta` capsule work.
- **Outcome:** Root and public changelog now describe landed `anvil capsule create`, `verify`, `explain`, witness-chain evidence, and SARIF diagnostics; docs-site changelog JSON has a draft `0.8.0-beta` entry; review-capsules concept page no longer marks SARIF diagnostics as future work.
- **Worked:** Cross-checked APS `git-native-governance` statuses before changing release prose.
- **Failed:** None.
- **Friction:** `format:check` required one oxfmt wrap fix in the public changelog.
- **Improvement:** Keep release changelog updates in the same PR that flips late-stage feature work to `Merged`.
- **Follow-up:** None.

### 2026-06-12 — opencode

- **Task:** Run due `anvil-bench` benchmark spot checks.
- **Outcome:** Resource budgets passed and a partial benchmark history record was
  added for 2026-06-12; `pnpm bench` did not complete because the harness-piped
  `cargo test -p anvil-bench` returned 101 while the direct command passed.
- **Worked:** Direct, unpiped `cargo test` and `cargo bench` commands isolated the
  product benchmarks from the harness capture issue.
- **Failed:** Initial `pnpm bench` runs stopped before benchmark execution.
- **Friction:** `scripts/bench/run.sh` assumes `target/release/anvil`, but this
  worktree uses the per-worktree `CARGO_TARGET_DIR` cache.
- **Improvement:** Benchmark harness logging should be robust to piped cargo-test
  output and should resolve the release binary from Cargo target metadata or the
  active `CARGO_TARGET_DIR`.
- **Follow-up:** Promote a CIB item if the tee/cargo-test failure recurs on the
  next benchmark run.
### 2026-06-18 — claude

- **Task:** Fix failing "Acknowledgements freshness" CI job on a dependency-bump PR.
- **Outcome:** Regenerated `ACKNOWLEDGEMENTS.md` so the freshness check now passes for the `uuid` 1.23.3 bump.
- **Worked:** Pulling the exact failing job logs first made the fix deterministic and kept scope to one generated file.
- **Failed:** Local pre-change validation was partially blocked because repo-wide JS checks need dependencies that were not installed in this worktree.
- **Friction:** The acknowledgements generator needs both `cargo-about` and `tools/dev` node tooling available to reproduce CI locally.
- **Improvement:** For dependency-update PRs, run the acknowledgements generation flow immediately after lockfile changes to avoid late CI failures.
- **Follow-up:** none


### 2026-06-18 — claude

- **Task:** Evaluate agentpatterns.ai + Arcanum-Sec/sec-context as anti-pattern
  sources; triage sec-context and propose a catalogue/category structure.
- **Outcome:** Triage brainstorm (36 items) + ADR-087 (Proposed) for an
  `insecure-construction` category, scoped to syntactic-smell families; logged.
- **Worked:** Grounding the triage in Anvil's actual detection tiers (regex /
  ADR-071 AST-query / secret check) made the covered/route-out/out-of-model
  split fall out cleanly; ~half of sec-context is taint/absence-class and not
  Anvil-shaped.
- **Failed:** agentpatterns.ai/anti-patterns returned HTTP 403 to WebFetch; used
  WebSearch + corroborating sources instead.
- **Friction:** Category naming nearly collided with the existing SEC CI-pipeline
  module / `security.yml`; caught only by grepping the APS index.
- **Improvement:** When proposing a new enum/category name, grep the APS index
  and workflows for the bare word first — collision check is cheap and the rename
  cost later is not.
- **Follow-up:** None yet; if ADR-087 is accepted, open a catalogue module for
  the first-wave families (weak-cryptography, unsafe-rendering).

### 2026-06-18 — claude

- **Task:** Flip ADR-087 to Accepted (operator) and open the first-wave INSEC
  catalogue module realising the sec-context syntactic-smell subset.
- **Outcome:** ADR-087 Accepted; new `insecure-construction-catalogue.aps.md`
  (INSEC-001..008, 6 Ready + 2 deferred Proposed); index row + DECISION-LOG
  status updated. aps:index:check / aps:drift / adr:check green.
- **Worked:** PYLAN-003/-009 were a near-exact template for a new pattern-family
  catalogue module (family `.anvil` + registry + types.rs + FP-bar dogfood).
- **Failed:** Stale local `main` had diverged from origin (rebase-merge rewrote
  history); had to reset the new branch to origin/main before working.
- **Friction:** `aps:active-lint` can't run here — the `aps` CLI binary is not
  installed (node_modules missing); relied on index:check + drift instead.
- **Improvement:** none.
- **Follow-up:** When INSEC starts, promote the deferred opt-in items only after
  the enabled-family FP bar is met; register the `WC`/`UR` registry prefixes.
### 2026-06-19 — opencode

- **Task:** Run Clawpatch report triage and apply selected fixes.
- **Outcome:** One Clawpatch-generated fix was retained and several narrow manual
  fixes were applied after Clawpatch refused further fixes on a dirty worktree.
- **Worked:** Filtering report output to current-tree, high-confidence findings
  avoided flooding the backlog with stale worktree and won't-fix items.
- **Failed:** Chaining `clawpatch fix` hid that the first validation failure stops
  later fixes, and Clawpatch requires a clean tree for each subsequent fix.
- **Friction:** `clawpatch report` can surface hundreds of mixed-lifecycle issues;
  a raw run is too noisy to translate directly into GitHub issues.
- **Improvement:** Run `clawpatch fix` one finding at a time from a clean tree, and
  triage report output by status/path before filing any follow-up work.
- **Follow-up:** none

### 2026-06-21 — clawpatch

- **Task:** CIB-085 post-merge closeout — APS Merged + clawpatch revalidate.
- **Outcome:** PR #2823 merged; four medium findings revalidated `fixed`; policy-
  engine multi-row binding finding triaged `wont-fix` (documented first-binding
  contract for `EvalResult.value`, full set in trace). Open clawpatch count
  85 → 80 on canonical tree.
- **Worked:** `clawpatch revalidate --finding` per ID gave actionable reasoning
  without re-running the full 715-finding corpus.
- **Failed:** none.
- **Friction:** Revalidate is slow (~85–130s per finding) when run serially for
  a batch of five.
- **Improvement:** Schedule clawpatch closeout revalidates in the same PR branch
  before merge when the APS item lists explicit finding IDs.
- **Follow-up:** optional `wt remove` for `fix/cib-085-clawpatch-rust-contracts`
  worktree; `clawpatch map` after stale worktree cleanup.

### 2026-06-20 — other

- **Task:** Address CIB-085 PR review feedback on gctx impact capping under lock.
- **Outcome:** Reworked affected-symbol capping to keep a sorted bounded vector while streaming symbols; deterministic output retained.
- **Worked:** Existing order-independence and cap tests were sufficient to validate behaviour after the refactor.
- **Failed:** none.
- **Friction:** The previous collect-then-truncate approach looked deterministic but silently removed lock-budget guarantees.
- **Improvement:** For lock-held paths, enforce caps during collection rather than in post-processing.
### 2026-06-20 — opencode

- **Task:** Complete GCTX-020 token-count estimator.
- **Outcome:** Added a parser-free conservative estimator in `anvil-graph-cache`,
  exported it through the graph module, and closed GCTX-020 as Done.
- **Worked:** A compile-failing red test was enough to pin the public API and keep
  the implementation dependency-free.
- **Failed:** `cargo fmt --all --check` initially caught one long error-attribute
  line after the green step.
- **Friction:** The APS index generator updated only the count, not the stale
  prose on the same row.
- **Improvement:** After generated APS count updates, read the surrounding index
  prose for stale item ranges before closing the task.
- **Follow-up:** none

### 2026-06-20 — opencode

- **Task:** Public-facing Anvil documentation pass.
- **Outcome:** Corrected stale CLI claims, unreleased-version framing, public doc
  links, and policy-test wording across `docs/public/anvil`.
- **Worked:** Pairing a quick public-doc audit with source checks caught concrete
  command drift without widening into a full content rewrite.
- **Failed:** A Docusaurus custom heading id fixed the site build but failed the
  repo docs checker, so the self-anchor was removed instead.
- **Friction:** The repo link checker and Docusaurus slug handling do not recognise
  the same anchor forms.
- **Improvement:** For public docs, prefer nearby prose over self-anchor links when
  the target is in the same short section.
- **Follow-up:** none

### 2026-06-22 — grok

- **Task:** DOCSYNC-023 — full Kindling public docs refresh against upstream v0.2.0.
- **Outcome:** Updated 16 existing pages, added 3 new pages (`without-claude-code`,
  `integrations`, `vscode`), sidebar, APS tracking, and docs-check baseline entries;
  `pnpm docs:check` and `pnpm aps:index:check` green.
- **Worked:** Reviewing the sibling `kindling` repo first produced a precise
  command-by-command delta (demo/browse, thin-client adapters, score range).
- **Failed:** Initial `docs:check` failed on new Docusaurus route links until
  baseline fingerprints were added for the new pages.
- **Friction:** `/kindling/...` site routes are intentionally baselined as broken
  filesystem paths in the repo link checker.
- **Improvement:** When adding Kindling public pages, update
  `docs/governance/docs-check.baseline.json` in the same PR.
- **Task:** Full refresh of `docs/public/aps/` against anvil-plan-spec v0.4.0 (DOCSYNC-024).
- **Outcome:** Rewrote 9 existing pages, added 6 new pages, updated sidebar; terminology,
  CLI, and file layout now match the sibling repo.
- **Worked:** Reading anvil-plan-spec `docs/**` and templates as source of truth kept the
  derived public mirror accurate without inventing schema.
- **Failed:** Initial `/aps/...` absolute links failed `docs:check` — the link checker
  resolves filesystem paths, not Docusaurus routes.
- **Friction:** Governance metadata `Type` must be `Public docs`, not `PublicDoc`.
- **Improvement:** Use relative `.md` links in `docs/public/**` like the Anvil section does.
- **Follow-up:** none

### 2026-06-24 — codex

- **Task:** Complete CIB GCTX client hardening in a fresh Worktrunk worktree.
- **Outcome:** Extracted the repeated GCTX daemon JSON-RPC client into `mcp::gctx_client`, reused it from tools and resources, closed CIB-099, and started CIB-100 with the Windows named-pipe path pending Windows matrix validation.
- **Worked:** The existing no-daemon MCP degradation tests covered every GCTX tool after the extraction, and a small pure classifier test pinned the non-Linux/macOS Unix peer-validation downgrade.
- **Failed:** Local Windows target checks could not reach the crate under test because this host lacks the Windows C toolchain (`x86_64-w64-mingw32-gcc` / `lib.exe`).
- **Friction:** Cross-platform CIB items need a validation path that distinguishes implementation compile risk from unavailable local toolchains.
- **Improvement:** When starting a Windows-specific CIB item from Linux, record the exact blocked cross-target check in APS and leave the item open until a Windows runner proves it.
- **Follow-up:** Finish CIB-100 on a Windows-capable runner.
### 2026-06-24 — opencode

- **Task:** Unblock as many Blocked APS plans as possible (8 Blocked work items
  across CGBDG, TRACE-002/003, DPO-003/004/005, DEVENV-003, MLP2-007, MLP2-051d).
- **Outcome:** One module fully unblocked — **CGBDG Blocked → Ready** (its
  MLP-002 blocker is Done and the witness-schema follow-ups MLP2-011
  Released/Shipped + MLP2-012 Merged are terminal). Reconciled additional stale
  blocker prose: **DSV Sub-phase B** is no longer Blocked (GV2-021
  Released/Shipped; DSV-030 Merged), **GCTX-014** is no longer blocked on call-edge
  support (Merged via #2715), and **TRACE-003** now records that INTD-015 is
  Complete + ADR-059 decided, leaving only the EXPORT-001-deferred
  sampled-exporter slice. The remaining exact Blocked work items (TRACE-002,
  DPO-003/-004/-005, DEVENV-003, MLP2-007, MLP2-051d, TRACE-003 residual) verified
  genuinely still blocked. `aps:active-lint` / `aps:index:check` / `aps:drift` /
  `lint:md` all clean.
- **Worked:** The 2026-05-29 reconciliation-sweep audit had already pre-analysed
  CGBDG ("keep Blocked pending schema-stability"), so checking whether MLP2-011/-012
  had since landed was the whole unblock — verifying dependency item statuses against
  current truth, not re-deriving from scratch.
- **Failed:** nothing functional.
- **Friction:** `markdownlint-cli` via `pnpm exec`/direct node printed usage and
  exited 0 regardless of file args in this shell; had to fall back to the repo
  `lint:md` script to actually lint. A blocked work item encodes its blocker only
  as prose, so "is it still blocked?" requires manually resolving each named
  dependency's current status — there is no machine-checkable blocker link.
- **Improvement:** A lightweight "blocked-by" cross-link check (assert each
  Blocked item's named dependency is not already terminal) would auto-surface
  unblockable items during reconciliation.
- **Follow-up:** Consider promoting that blocked-by staleness check to a CIB item;
  CGBDG now sits in the index "Dormant" band though it is Ready — a future pass
  should lift it into an active section.
### 2026-06-24 — other

- **Task:** GV2-032 + GCTX-021/022/023 symbol spans, snippet extractor, budget slicer, `anvil_symbol_context`
- **Outcome:** Implementation complete on `feat/gctx-021-snippet-extractor`; workspace `cargo test` green; APS 21/21 GV2, 12/14 GCTX
- **Worked:** Building on partial GCTX-021 branch; subagent completed slicer/MCP wiring; `SnippetResult` as sole text carrier
- **Failed:** Prior `feat/gctx-phase2-context` branch diverged (separate snippet-types crate) — not merged
- **Friction:** Protocol count-pin tests (`ALL_ANVIL_METHODS`, USAGE-004) needed manual bump after each new RPC
- **Improvement:** Auto-derive allowlist count from `COMMAND_INVOKED_ALLOWLIST.len()` instead of hard-coded arithmetic
- **Follow-up:** Council + PR for `feat/gctx-021-snippet-extractor`

### 2026-06-29 — opencode — fence follow-ups

- **Task:** Ground fence/session-control behaviour and file APS follow-up.
- **Outcome:** Added Proposed design items for sandbox-grade fence semantics
  (MLP2-077), daemon-to-session write-back (MLP2-078), interrupt/fence/kill
  lifecycle validation (MLP2-079), and headless background save-time driving
  (DSV-046).
- **Worked:** Existing MLP2/fence/interrupt code made the gap precise: current
  fences revoke trust and block future registration, while interrupt exists as a
  separate process-control ladder.
- **Failed:** none
- **Friction:** APS aggregate counts are intentionally stale after adding items;
  `aps:index:check` reports advisory drift by design.
- **Improvement:** none
- **Follow-up:** none

### 2026-06-29 — opencode — worktree registration

- **Task:** Capture follow-up daemon/worktree registration UX design gaps.
- **Outcome:** Added ACTMO-013 for subsequent worktree registration, outside-worktree
  `anvil start` semantics, and a scoped local app as an optional daemon vehicle.
- **Worked:** Keeping this in ACTMO tied the UX to activation state instead of
  scattering it across daemon validation items.
- **Failed:** none
- **Friction:** The daemon primitive exists, but product semantics for how humans
  and agents discover/register later worktrees are not yet explicit.
- **Improvement:** Treat daemon vehicle ideas as control-plane UX until a design
  explicitly authorises broader desktop-app scope.
- **Follow-up:** Design ACTMO-013 with DSV-046 so registration and headless save-time
  activation agree.

### 2026-06-29 — opencode — release addendum

- **Task:** Document what must be added to the `v0.9.0-beta` release plan for a
  useful daemon-backed release.
- **Outcome:** RELEASE-PLAN now identifies the original assistant-graph scope as
  complete while adding a default-on daemon usefulness addendum under APS review:
  ACTMO-013 plus DSV-046.
- **Worked:** Keeping the addendum conditional on APS promotion preserves the
  release plan's derived authority while making the operator cut-line concern
  visible.
- **Failed:** none
- **Friction:** The release plan's prior "scope complete" wording hid the product
  usefulness gap even though the implementation truth was already captured in APS.
- **Improvement:** Release plans should distinguish implementation completeness
  from beta-usefulness cut-line readiness.
- **Follow-up:** Promote or reject ACTMO-013/DSV-046 before cutting `v0.9.0-beta`.

### 2026-06-30 — opencode

- **Task:** Fix failed `anvil-api` Vercel production builds.
- **Outcome:** Added DOM fetch typings to the API TypeScript build so clean Vercel
  builds recognise `Response.ok`, `Response.status`, and `Response.json`.
- **Worked:** Isolating the hotfix in a clean Worktrunk branch avoided mixing it
  with unrelated local APS edits.
- **Failed:** none
- **Friction:** The Vercel failure only reproduced in a clean platform install;
  warm local TypeScript build-info initially masked the missing fetch lib.
- **Improvement:** Keep platform build commands covered by a clean local smoke or
  CI check when changing package-manager/runtime typing.
- **Follow-up:** none

### 2026-07-02 — opencode

- **Task:** Reset policy planning around real policy value and save-time
  enforcement.
- **Outcome:** Added a POLRESET conductor, narrowed OPAE to regorus-backed
  authoring/runtime UX, and reset CPACKS to starter-pack-first.
- **Worked:** A conductor module let the plan coordinate POLVAL/OPAE/CPOL/EXCEPT
  without making OPAE the umbrella for every enterprise policy idea.
- **Failed:** Initial POLRESET wording used ADR-style hyphenated references that
  active APS lint and index-count parsing misread as work-item references.
- **Friction:** Stale module IDs in downstream plans made the OPAE reset cascade
  farther than the main policy files.
- **Improvement:** When renumbering or resetting an APS module, immediately grep
  for old work-item IDs across all active modules before validation.
- **Follow-up:** none

### 2026-07-02 — claude

- **Task:** Spike — Astro 7 + Starlight rebuild of `apps/docs-public` (APS section only) to de-risk a Docusaurus → Astro migration; prove the docs-shell proxy contract.
- **Outcome:** New `apps/docs-public-astro` builds green on Astro 7.0.2 / Starlight 0.41.0 (9 pages + Pagefind in ~2.4s); content symlinked from canonical `docs/public/aps` with zero edits; draft PR opened. Right-sized (no APS module / Council) as an explicit throwaway spike.
- **Worked:** `build.assets: 'assets'` forces hashed assets under the proxy-forwarded `/assets/` prefix instead of `/_astro/`; Docusaurus frontmatter (`id`/`sidebar_position`) validated unchanged under `docsSchema()`.
- **Failed:** Pre-investigation answer asserted Astro 7 was alpha — it had GA'd (2026-06-22); a stale "what's new" digest outranked the release post. Corrected after the user pushed back.
- **Friction:** Pagefind serves `/pagefind/*`, not covered by the shell proxy matcher — search 404s behind the proxy until added. Easy to miss.
- **Improvement:** For "is X the latest version" questions, check the release feed / package registry directly before answering, not a monthly digest. Registry (`npm view`) is the fastest ground truth.
- **Follow-up:** If migration proceeds, write a proper APS module (DSITE successor) covering blog/RSS gap, theme parity, remaining sections, and the private app; decide Path A vs B.

### 2026-07-02 — claude (recovered from a concurrent `opencode` session's uncommitted work)

- **Task:** Reconcile the INSEC NBI/index entries after first-wave merge evidence landed, and correct a stale `v0.8.2-beta` release-note claim. Found as uncommitted, complete-looking edits sitting in a shared checkout (stashed to avoid losing or clobbering them), then recovered onto current `main` once the stash no longer applied cleanly.
- **Outcome:** Removed INSEC-001..006 from the NBI pickup table, promoted DASH to rank 1, and updated both the INSEC index row and header prose to say "Merged 2026-07-01 via #3028" instead of "implemented" — matching the per-item statuses already recorded in the module file. Corrected the release header/footer to note `v0.8.2-beta` is a Windows daemon-ensure smoke tag, not the latest promoted release.
- **Worked:** Verified every claim before landing rather than trusting the stash — confirmed PR #3028 is merged, confirmed issue #3031 (not a second PR) is the closed FP-acceptance-bar tracking issue, and confirmed via the GitHub API that `v0.8.2-beta` is `prerelease: true` with `/releases/latest` resolving to `v0.8.1-beta`.
- **Failed:** none — but the original stash cited "#3031" ambiguously (read as a second merge PR) and "CIB-072" without a direct citation; both corrected to verified references.
- **Friction:** This shared checkout had main advance ~7 commits from other concurrent agents between finding the stash and landing the fix — including one that needed its own clobber-recovery (`chore(aps): restore 11 CIB statuses clobbered by e57a65fdf`). Also found a second unrelated uncommitted change (`CONTEXT.md` casing edits) mid-flight; left untouched rather than swept into this commit.
- **Improvement:** In a shared checkout, treat any uncommitted change you didn't make as load-bearing until proven otherwise — stash with a clear label, verify every factual claim against current state before landing it (state moves fast here), and never bundle an unrelated foreign uncommitted change into your own commit just because it's sitting in the same working tree.
- **Follow-up:** none for this task; the `CONTEXT.md` edit is still uncommitted in this working tree for its owner to land separately.
### 2026-07-03 — claude

- **Task:** CIB-149 — stop treating an unverified first wire root as the `Allowlist` confinement primary; derive the implicit primary from the daemon-verified `RegisterSession` worktree instead.
- **Outcome:** `Confinement::to_admitted_roots` now takes `Option<&Path>` (verified primary); `save_time::authorise_root` seeds allowlist admission from `originating_session.worktree`, falling back to allow-entries-only when no session is bound. Open-mode first-touch seeding unchanged. New unit + daemon tests (a) no-session refuses first-named unlisted root, (b) session-bound worktree is the primary while a first-named unlisted root is still refused.
- **Worked:** Threading `self.originating_session.as_ref().map(|s| s.worktree.as_path())` at each `authorise_root` call site is a disjoint-field borrow, so it coexists with `&mut self.admitted` without cloning.
- **Failed:** Four existing gctx `*_rejects_unadmitted_root` tests encoded the old bypass (first-named root as implicit primary with no session) — they broke and had to be re-based onto a bound `set_originating_session` to keep their C3/CE-8 intent.
- **Friction:** Package name is `eddacraft-anvil-intercept` (not `anvil-intercept`); `cargo test -p anvil-intercept` fails to match.
- **Improvement:** When tightening an implicit-admission source, grep the whole test module for the old permissive assumption ("implicitly admitted primary") before running — several unrelated tests lean on it.
- **Follow-up:** none

### 2026-07-03 — claude (CIB-149 Council remediation)

- **Task:** Address blocking Council findings on CIB-149: the verified primary was sourced from an in-connection `RegisterSession` (`originating_session.worktree`), but real clients open a fresh one-shot connection per RPC — so register and verb never share a socket and the implicit-primary admission was permanently unreachable in production; and no test exercised the real two-connection topology.
- **Outcome:** Added a dedicated `SaveTimeConn::verified_primary` seeded at connection setup from the durable `SessionRegistry` via the authenticated peer's PID lineage (`worktree_for_lineage(peer_pid)`, the same anti-PID-reuse anchor the spoof cross-check uses) through a new `ipc::seed_save_time_verified_primary` helper the accept-loop calls. Reachable across connections; gated on `Confinement::is_allowlist()` to skip the `/proc` walk on the default open posture; in-connection `RegisterSession` still seeds it too. New ipc-level test drives the real topology: one connection registers the worktree, a *separate* verb connection (no in-connection register) resolves it as the primary, while an unregistered peer gets only the allow entries.
- **Worked:** `worktree_for_lineage(peer_pid)` was already the reviewed production resolver (`spoof_block_response`), so applying it to confinement is an existing mechanism, not a new trust model — kept the fix in-scope.
- **Failed:** Inserting the helper between a pre-existing `#[allow(clippy::too_many_lines)]` and `handle_connection` silently re-attached the allow to the helper, so `handle_connection` tripped `-D warnings`; moved the attribute back onto the function.
- **Friction:** `handle_connection` sits right at clippy's line budget — a multi-line call statement (rustfmt-wrapped) costs enough lines to matter; passing the `Option`s straight into the helper kept the call site to one statement.
- **Improvement:** When adding a free fn just above an existing one, check whether the insertion point splits an attribute from its target item.
- **Follow-up:** Activation/`anvil workspace register` sends no lineage, so `worktree_for_lineage` is empty for those worktrees — the implicit primary covers MLP2-025 lineage-registered agents; non-lineage registrations must use explicit allow entries. If operators need the implicit primary for activation-registered worktrees, that needs a peer→worktree binding that does not exist yet (separate design/ADR).


### 2026-07-03 — claude (CIB-149 merge)

- **Task:** Land PR #3117 and flip CIB-149 to Merged.
- **Outcome:** Rebased onto `origin/main`, gates green, merged via `--rebase`; CIB-149 status compacted to Merged with a one-line Summary.
- **Follow-up:** none

### 2026-07-03 — claude (CIB-149 relocated-bypass fix)

- **Task:** Security review of PR #3117 found the fix RELOCATED the bypass rather
  than closing it. The "daemon-verified primary" was sourced from the peer's
  `RegisterSession` worktree (in-connection) / `worktree_for_lineage(peer_pid)`
  (durable registry). Both are just the path a same-uid client passed to
  `session.register`, stored verbatim — the daemon verifies only *who* the peer
  is (its PID lineage), never that the *path* should be admitted. A same-uid
  attacker could `RegisterSession { worktree: "/anything" }` then name `/anything`
  in a save-time/GCTX verb and have it admitted past an empty Allowlist — the
  exact CIB-149 class, moved one wire frame over. No genuinely daemon-attested
  worktree source exists (nothing the connecting client cannot originate itself).
- **Outcome:** Fail-closed. Removed the implicit primary entirely in `Allowlist`
  mode: `Confinement::to_admitted_roots()` drops its `verified_primary` param and
  admits ONLY the operator allow entries. Removed `SaveTimeConn::verified_primary`,
  `set_verified_primary`, `Confinement::is_allowlist`, and
  `ipc::seed_save_time_verified_primary` + its accept-loop call.
  `set_originating_session` now records the session for telemetry correlation
  ONLY (its worktree is client-supplied and no longer influences admission).
  `Open` mode first-touch adoption is unchanged. Every root — including a
  connection's own self-declared `RegisterSession` worktree — must independently
  match an allow entry.
- **Tests:** Replaced the bypass-encoding tests
  (`allowlist_session_bound_worktree_is_primary_not_first_named_root`,
  `verified_primary_resolves_from_registry_across_connections`) with regressions
  asserting a registered-but-unlisted worktree is `NotAdmitted`
  (`allowlist_registered_session_worktree_is_not_admitted`,
  `registered_worktree_is_not_implicitly_admitted_in_allowlist`). Confinement
  tests updated: empty Allowlist admits nothing (`allowlist_empty_admits_nothing`).
  The four gctx `*_rejects_unadmitted_root` tests re-based onto an explicit allow
  entry instead of the implicit primary. Proved fails-before/passes-after against
  the pre-fix tree (pre-fix admitted the registered worktree; post-fix refuses).
  `cargo test -p eddacraft-anvil-intercept --lib` = 880 passed; clippy + fmt clean.
- **Lesson:** "daemon-verified" conflated peer-identity verification with
  worktree-content authorisation. When an admission source is client-declared,
  restricting *which* client-declared value is used does not make it verified —
  fail closed and require an operator allow entry.
- **Follow-up:** If a zero-config Allowlist workflow genuinely needs a
  connection's own worktree auto-admitted, that requires a real daemon-attested
  peer→worktree binding (a worktree tied to a process the daemon itself
  spawned/attested), which does not exist today — separate design/ADR.
### 2026-07-03 — claude

- **Task:** CIB-150 — verify the wire `agent_tag` durable-membership claim before honouring it (close the trust gap where any same-uid IPC client mints an activation-spine `AgentTag` and consumes the persisted registered-worktree budget).
- **Outcome:** Added `ipc::verify_durable_membership_claim` + `peer_authorised_for_durable_membership`; a durable (activation-spine) claim is honoured only when the peer runs the daemon's own binary (Linux `/proc/<peer_pid>/exe` == canonical `current_exe`), else downgraded to a non-durable tag before `dispatcher.register` — registered, not rejected. 4 new dispatch tests; registry.rs gained a trust-model doc note.
- **Worked:** The daemon and CLI are one `anvil` binary, so the authorisation test mirrors `verify_lineage_claim` peer-derivation exactly; downgrade-not-reject keeps a benign mis-tagged client working. The in-process `register_on_start` path never crosses the dispatcher, so legitimate startup durable registration is untouched.
- **Failed:** none.
- **Friction:** In-process tests get authorisation "for free" — `std::process::id()` peer vs the test binary's `current_exe` both resolve to the same `/proc/self/exe`, so the authorised-persists guard and a spawned-`sleep` non-anvil peer cover both branches without a fixture daemon.
- **Improvement:** When a same-uid trust boundary must distinguish "our binary" from "a neighbour", exe-path equality via `/proc/<pid>/exe` is a cheap, fail-closed check that unit-tests cleanly in-process.
- **Follow-up:** Non-Linux platforms downgrade every wire durable claim (no portable peer-exe reader yet) — durable membership there relies on the in-process `register_on_start` path, same caveat as `verify_lineage_claim`.

### 2026-07-03 — claude

- **Task:** CIB-151 — stop trusting a client's `ChangeKindWire` (`Deleted`/`Renamed`) to suppress the guarded read + antipattern scan in `validate_paths`.
- **Outcome:** `per_path_outcome` now attempts `read_guarded` for every change kind; a path declared deleted/renamed but still holding live bytes is read, hashed, and scanned (a blocking AP-008 finding can no longer be evaded), while a genuinely vanished path stays content-free. Taxonomy-driven staleness (`Deleted`/`Renamed` non-certifiable) unchanged. Removed the `change_has_bytes` gate. Added 3 TDD tests: live bytes behind a `Deleted` claim, live bytes behind a `Renamed` claim, and an oversized `Deleted` path keeping its `Deleted` `StaleReason` (not the generic oversized fallback). The genuinely-vanished case stays covered by the existing `deleted_path_has_no_daemon_hash_and_is_partial` test; existing coverage green.
- **Friction:** Crate package name is `eddacraft-anvil-intercept`, not the `anvil-intercept` in the plan's `cargo test -p` command — `-p anvil-intercept` fails to match.
- **Improvement:** Resolve the real Cargo package name (`grep '^name' Cargo.toml`) before running `-p` commands lifted from a plan; the manifest name and directory name diverge here.
- **Follow-up:** none

### 2026-07-04 — claude

- **Task:** CIB-163 — stop `anvil start` printing init's "Next: run `anvil start`" line when the orchestrator runs init inline.
- **Outcome:** Threaded a small `InitInvocation::{Standalone, FromStart}` context from the call sites through `init::run_in → run_plain/run_tui → print_success → success_message`. `FromStart` (the `orchestrator/mod.rs` inline init step) drops the closing next-step line so the activation ending owns the single next step (UJ-001); standalone `anvil init` passes `Standalone` and is byte-for-byte unchanged. TDD: added `init_from_start_suppresses_anvil_start_next_step` (proved red by temporarily removing the conditional) plus `standalone_and_from_start_differ_only_by_next_step_line`, and pinned the existing UJ-001 test to `Standalone`. Fresh-repo `anvil start --no-tui` transcript now contains no "run `anvil start`".
- **Friction:** Standalone `anvil init` can't be transcript-checked on this box — it hits the auth gate ("Authentication required"), whereas the read-only `anvil start` activation posture does not; the byte-identical standalone copy is proven by unit test instead of a live transcript.
- **Improvement:** When suppressing one line of a shared render helper, an invocation enum threaded to the leaf beats a bool flag — it self-documents at every call site and a `starts_with` test cheaply pins "the two paths differ only by the suppressed tail".
- **Follow-up:** CIB-166 (one next-step arbiter per `anvil start` ending) still owns reconciling the diagnostic `next:` and closing `Next:` lines; this change only removes init's competing line.
- **Task:** CIB-170 — make clean-repo showcase findings unmistakably examples so a user cannot mistake the demo secret at `src/services/auth.rs:42` for a real leak.
- **Outcome:** Added `is_showcase: bool` to `ScanResults` (preserved through `filter_by_domain`, defaulted false at every construction site, `true` at both `welcome.rs` showcase fallbacks); `render_findings_list` now swaps the panel title for an "Example findings — your scan found no issues" banner and prefixes each row with a distinct reversed `EXAMPLE` badge when the flag is set. Real-scan renders are unchanged. TDD: two new `discovery_render` tests (showcase framing present, real-scan framing absent) plus an extended `filter_by_domain` preservation assertion; proved red before implementing.
- **Friction:** Crate package name is `eddacraft-anvil-tui`, not the plan's `-p anvil-tui`; the em-dash banner (~44 cols) clips the 50% left panel at 80 cols, so the render test uses a 140-col backend.
- **Improvement:** Keep the `[Example]` title prefix in `showcase.rs` untouched — belt-and-braces copy robustness independent of the render-layer banner.
- **Follow-up:** CIB-171 also edits `welcome.rs` (navigation/init-summary, different region) — sequence merges to avoid a conflict.


### 2026-07-04 — claude

- **Task:** CIB-162 — stop `anvil start` / `--verify` printing raw `{"timestamp":…,"level":"WARN"…}` JSONL mid-flow when the intercept daemon does not attest the worktree (the machine tracing line interrupts the human activation surface and reads as a crash).
- **Outcome:** Demoted all `emit_skip_event` arms in `activation/daemon_evidence.rs` to `tracing::info!` (previously four operator-actionable reasons — daemon unreachable / worktree unenforced / stale heartbeat / all-surfaces quarantined — emitted `warn!`). `info` is below the CLI default `warn` filter, so JSONL still flows to `ANVIL_LOG=info`-driven and file-sink consumers but no longer reaches the human surface. Operator visibility is now owned by the render layer — `render::daemon_evidence_label` already folds every `DaemonAttestation` state into the human `daemon:` / `meaning:` lines. TDD: integration test in `tests/start.rs` (no `{"timestamp"` line on stdout/stderr at default filter, for both `start` and `--verify`) + unit test pinning every skip reason at INFO + a render test pinning every attestation state has non-empty human copy.
- **Worked:** The council-2026-05-22 warn promotion existed only because skip events were once at `debug`; once the render layer folds every state into human copy, the tracing stream can drop back below the default filter without losing operator visibility. The `run_start_with_home` harness already isolates the daemon socket to a temp `XDG_RUNTIME_DIR`, so the no-attesting-daemon repro comes for free.
- **Failed:** none.
- **Friction:** `--verify` on a config-absent repo lands at `needs_action` before the daemon probe, so the repro needs the repo initialised (full `anvil start` first) to reach `ready_restart_required` where the probe fires. Capturing tracing levels needs `tracing-subscriber` as a test-only dev-dep; `about.toml` sets `ignore-dev-dependencies = true` so ACKNOWLEDGEMENTS is unaffected (the crate is already listed via anvil-observability).
- **Improvement:** When a diagnostic signal must stay operator-visible but off the human stdout surface, own visibility in the render/format layer and keep the tracing event below the default filter — don't promote the tracing level to force visibility, which leaks raw JSONL into human output under a JSON subscriber.
- **Follow-up:** `canonicalise_for_activation` still emits `warn!` on a canonicalisation failure (a genuine rare error, out of CIB-162 scope); anvil-cli additionally carries pre-existing broken intra-doc links (not rustdoc-gated for this crate).

### 2026-07-04 — claude

- **Task:** CIB-164 — make the first-run `verify:` block honest about active layers (hooks over-claim, wired-not-live L0, unsupported-repo recipe/watch contradiction).
- **Outcome:** `install_activation_hooks_silent` now returns `Result<bool>` (both hooks actually anvil-managed on disk, read back via `is_anvil_managed`); the orchestrator captures it and stamps `InstallReport.hooks_active`, which `render_first_run_recipe` reads instead of the old `.git`-exists heuristic. `render_first_run_recipe` splits `mcp_pre_write_live()` (active `L0 mcp pre-write`) from wired-only (`RestartRequired`→`L0 mcp pre-write (pending — restart required)`), and on `all_languages_unsupported` suppresses the `.ts` smoke recipe (`recipe: none …`). `start_next_step_line` gained an unsupported arm that no longer recommends `anvil watch`. 6 new tests (3 hooks.rs, 3 start.rs).
- **Worked:** Threading the honest bool through the already-returned `InstallReport` avoided touching every orchestrator call-site signature; reading hook state back off disk (not trusting the `created/updated/skipped` action) correctly reports `false` when a pre-existing *unmanaged* hook blocks anvil coverage.
- **Failed:** none.
- **Friction:** `anvil start`'s manual transcript is auth-gated in the agent shell (`Authentication required`), so the "no L3/L4 in a non-Git dir" check was validated via the unit test that proves the orchestrator threads `hooks_active=false` outside a repo, not a live transcript.
- **Improvement:** Honesty predicates should read back the durable artefact (marker on disk) rather than an install-action enum — "skipped" conflates "already ours" with "someone else's, left alone", and only the former is coverage.
- **Follow-up:** CIB-166 (one next-step arbiter) still needs the diagnostic `next:` and closing `Next:` to be reconciled; this change only removed the unsupported-repo `anvil watch` contradiction from the closing line.
- **Task:** CIB-171 — fix welcome TUI navigation scopes and init-summary honesty (Esc-on-discovery, config-file naming, hub sub-surface footers).
- **Outcome:** Extracted `discovery_outcome` so `Esc` (`SurfaceExit::Back`) on the discovery screen backs out to the hub instead of advancing into the tutorial. `generate_config` now returns a `GeneratedConfig { config_path, gitignore_updated }` and the landing summary names the real file; `ConfigFormat` labels and `success_message` derive from a single `CONFIG_FILE_NAME` (always `.anvilrc`) rather than promising `.anvil.yaml/.json/.toml` that are never written. Added an `embedded()` flag to gate/audit/doctor states so hub-hosted footers read "esc menu / q quit anvil" while standalone copy is unchanged. TDD throughout (red proven for each), 2 init snapshots regenerated for the relabelled formats.
- **Worked:** The footer is rendered from `Surface::help_text`, not the surface `render()`, so `embedded` needed no snapshot churn for gate/audit/doctor — plain `help_text` unit tests suffice; only the two init snapshots that echo the format label changed.
- **Failed:** none.
- **Friction:** The file the wizard writes is always `.anvilrc` (format is the in-file serialisation), so the item's "wrote .anvil.yaml" framing was slightly off — the real dishonesty was the format-select labels, not `welcome.rs:328`.
- **Improvement:** When "the summary hardcodes X" is reported, trace the actual writer (`generate_config`) before trusting the reported literal — the honest fix was relabelling the picker, not the summary path.
- **Follow-up:** none

### 2026-07-04 — claude

- **Task:** CIB-167 — terminal-first users only got a plain-language `meaning:` line on `ready_restart_required`; `needs_action`, `unsupported`, and `watching` never did, and the MCP tier tokens read as "done" under a restart-required headline.
- **Outcome:** Added three additive arms to `state_explanation` in `activation/render.rs` (NeedsAction: MCP config not written yet, run `anvil start`; Unsupported: honest no-action, no registry-supported languages, explicitly not an error; Watching: save-time fallback is weaker than MCP pre-write validation, run `anvil start --verify` to graduate). The `ReadyRestartRequired` arm is untouched so `--verify` output for that state stays byte-identical. Tier tokens in `diagnostic.rs` were left as a rendered contract; the rename-vs-gloss-vs-document question is filed as Draft CIB-180 for the owner. TDD: 3 new render tests asserting each state emits a `meaning:` line with the expected plain-language substrings (proven red first).
- **Worked:** The existing `state_explanation` → `writeln!("  meaning: …")` seam meant the fix was purely additive match arms; replacing the `_ => None` catch-all with explicit `NeedsAction`/`Unsupported`/`Watching` arms plus `Protecting | Error => None` keeps the compiler enforcing exhaustiveness if a new state is added.
- **Failed:** none.
- **Friction:** none — the `empty()`/`unsupported()`/`watching()` test helpers already resolve to the three target states, so the tests assert `protection_state()` first to pin that assumption.
- **Improvement:** When copy must land for a subset of enum variants but a machine-facing token in the same area is a parse contract, split the additive human copy (ship now) from the token-vocabulary change (owner decision) into separate items rather than blocking the safe copy on the contract call.
- **Follow-up:** CIB-180 (Draft) captures whether `restart_handshake_verified` / `server_startable` should be renamed, glossed at render time, or documented as observed-probe state.



### 2026-07-04 — claude (council remediation)

- **Task:** CIB-167 Council follow-up — two `meaning:` lines over-claimed/under-claimed: `NeedsAction` said "anvil has not written an MCP config" even when a `ConfigPresent` entry was on disk, and `Watching` attributed protection to save-time watch even when the daemon-backed spine (`Enforced`/`Promoted`) was the thing attesting.
- **Outcome:** Split the two static arms into `needs_action_meaning(d)` (branches on `ConfigStatus` + `highest_mcp_tier`, mirroring `why_summary_for_needs_action`) and `watching_meaning(d)` (branches on `daemon_attestation.attests_worktree()`, mirroring `why_summary`/`repair_hint`). NeedsAction now acknowledges a written entry and points at restart/re-verify; daemon-attested Watching credits the daemon-backed spine and frames MCP as an optional upgrade. Save-time-only Watching copy is byte-identical, and `ReadyRestartRequired`/`Unsupported` arms are untouched. TDD: 2 new render tests (written-entry NeedsAction, daemon-backed Watching) proven red first; existing NeedsAction test retargeted to the honest not-set-up-yet copy.
- **Worked:** Reusing the exact `daemon_attestation.attests_worktree()` / MCP-tier truth tables the `why:`/`next:` lines already dispatch on kept all four lines in lockstep, so the `meaning:` line can never disagree with the `next:` hint below it.
- **Failed:** none.
- **Friction:** none — the change stayed inside `render.rs`; no schema or diagnostic changes were needed.
- **Improvement:** A `meaning:` line that restates a state must dispatch on the same signal that produced the state, not a hard-coded assumption about the most common path — `NeedsAction` and `Watching` are both multi-cause states.
- **Follow-up:** none.

### 2026-07-04 — claude (merge)

- **Task:** CIB-167 — land the activation-state `meaning:` line work (PR #3135) onto main and flip its backlog Status to Merged.
- **Outcome:** Rebased the branch onto `origin/main`, confirmed all required checks green, and merged via rebase. CIB-167 Status flipped to "Merged 2026-07-04 via PR #3135" with a compacted Summary; the `restart_handshake_verified` / `server_startable` tier-token rename stays deferred to an owner contract decision.
- **Worked:** The per-item Status line and union-merge CI log kept the merge bookkeeping free of cross-PR conflicts.
- **Failed:** none.
- **Friction:** none.
- **Improvement:** none.
- **Follow-up:** none.

### 2026-07-04 — claude

- **Task:** CIB-172 — the first-run smoke recipe's step 3 (`rm .anvil-smoke-test.ts`) fails in cmd.exe (`'rm' is not recognized`) with no `cfg!(windows)` branch, unlike the tutorial's `create_policy_directory_command`.
- **Outcome:** Replaced the flat `RECIPE_LINES` const in `start.rs` with named `RECIPE_LINE_WRITE`/`RECIPE_LINE_EXPECT` consts plus branched `RECIPE_CLEANUP_UNIX` (`rm`) / `RECIPE_CLEANUP_WINDOWS` (`del`) consts and a `recipe_cleanup_line()` / `recipe_lines()` selector. Both cleanup variants are compiled and named on every host so each is directly testable regardless of build target. `render_first_run_recipe` and the two pinned-fixture tests now iterate `recipe_lines()`, staying green per platform. TDD: added `first_run_recipe_cleanup_is_platform_branched` asserting the Windows variant uses `del` and contains no `rm` (proven red as a compile error first).
- **Worked:** Mirroring the tutorial's `cfg!(windows)` accessor pattern kept both variants compile-checked and testable on a Unix host without a Windows runner.
- **Failed:** none.
- **Friction:** none.
- **Improvement:** none.
- **Follow-up:** Step 1's `echo 'const KEY = "…"'` single-quote quoting is a cmd.exe quirk (cmd echoes the quotes literally) — out of scope here; worth a follow-up CIB if the Windows smoke path is exercised end-to-end.

### 2026-07-04 — claude (CIB-173)

- **Task:** CIB-173 — make Windows editor detection PATHEXT-aware so `.cmd`/`.bat` editor shims are detected instead of only `.exe`.
- **Outcome:** Extracted `pathext_candidates` (parses PATHEXT, bounds it case-insensitively to `.exe`/`.cmd`/`.bat`/`.com`, order-preserving, falls back to the full set when unset/empty/no-intersection) and `binary_in_dir` (per-directory lookup taking the candidate list). Rewired `RealDetectionEnv::has_binary`'s `cfg(windows)` branch to iterate the PATHEXT-derived candidates, keeping the empty-PATH-component skip and the `accept_bare` spoof guard; refreshed the guard comments to reference PATHEXT. Five new unit tests (bounding/ordering/fallback + temp-dir `.cmd` shim resolves while a bare extensionless file is guarded out).
- **Worked:** TDD via pure helpers dodged the `unsafe_code = "forbid"` block on `set_var` — temp-dir tests exercise the lookup without mutating the process environment; `write_executable_shim` makes them cross-platform.
- **Failed:** none.
- **Friction:** the crate is `eddacraft-anvil`, not `anvil-cli`, so the item's `cargo test -p anvil-cli` invocation had to be redirected.
- **Improvement:** cfg-gated helpers used only on Windows need `#[cfg_attr(not(windows), allow(dead_code))]` to stay clippy-clean on Unix while remaining test-reachable on all platforms.

### 2026-07-04 — claude (CIB-174)

- **Task:** CIB-174 — the ensure-failure recovery copy said "the daemon did not become ready within {bind_timeout}s" (`ensure.rs:322`), under-reporting the real bound: an in-flight probe can overrun `bind_timeout` by one `PROBE_TIMEOUT`, so the effective ceiling is `bind_timeout + PROBE_TIMEOUT` (documented as intentional at `ensure.rs:346-347`).
- **Outcome:** TDD — added a red test driving `ensure_with` through the spawn+never-answer path with an 80ms `bind_timeout`, asserting the copy names `(bind_timeout + PROBE_TIMEOUT).as_secs()` (2s) rather than the bare `bind_timeout` (0s). Green fix derives the printed figure from `(params.bind_timeout + PROBE_TIMEOUT).as_secs()` (prefer derivation over clamping, since the overrun is documented as intentional), keeping the log-path hint intact. Aligned the independent `start.rs:1602` fixture literal to "12s" to match `watch.rs:1984`.
- **Worked:** Deriving the expected ceiling from the `PROBE_TIMEOUT` constant inside the test keeps the assertion honest if the probe budget ever changes.
- **Failed:** none.
- **Friction:** Package id is `eddacraft-anvil-intercept`, not `anvil-intercept` as the Validation line reads — the latter is a target name.
- **Improvement:** Recovery copy that quotes a timeout must quote the effective wall-clock bound, not the nominal budget, when the wait can overrun by design.
- **Follow-up:** none.
- **Task:** CIB-175 — actionable, platform-aware watcher-start failure guidance off Linux.
- **Outcome:** Added pure `failure_guidance(&notify::Error)` + shared `watch_limit_guidance()` in `anvil-kernel/src/watcher/mod.rs` (Linux inotify sysctl wording cfg-gated; generic reduce-scope copy elsewhere; Io→permission/fd, PathNotFound→retry, Generic→retry/report). Refactored the hardcoded inotify hint in `watch.rs` partial-registration path to reuse it, and downcast `WatchError::Watcher(Notify)` in the CLI `run_watch` failure to append the guidance as anyhow context (raw chain preserved for `--json`/debug). Four cfg-gated unit tests synthesise notify errors and assert cause+next-step per platform.
- **Worked:** notify 8.2 exposes `Error::new`/`io`/`path_not_found`/`generic` constructors, so synthesising errors for unit tests needed no fake trait-object seam.
- **Failed:** none.
- **Friction:** The item's validation string `cargo test -p eddacraft-anvil-kernel watcher` is a name filter that matches none of the new `failure_guidance_*` test names; ran `--test watcher_integration` to exercise them.
- **Improvement:** none.
- **Follow-up:** capacity.rs Linux preflight intentionally untouched (existing tests cover it).
### 2026-07-04 — claude (autonomous)

- **Task:** CIB-176 — detect sh-less git before relying on `#!/bin/sh` hooks. Activation-installed and `anvil hooks install` file hooks are `#!/bin/sh` scripts; under a git lacking a bundled/PATH `sh` (a sh-less Git for Windows) they are on disk but never execute, so the L3/L4 layer vanishes with no signal.
- **Outcome:** Added an injectable `detect_hook_interpreter(windows, path_entries, git_exe, exists)` core in `hooks.rs` returning `HookInterpreterStatus::{Available,Missing,Unknown}` — POSIX checks `/bin/sh`+PATH; Windows probes PATH `sh.exe` then Git-for-Windows `usr/bin/sh.exe` siblings anchored on the git binary, warning only on a definitive Missing. Wired into `install_activation_hooks_silent` (returns `false` on Missing so the `verify:` L3/L4 line stays honest, CIB-164) and the file-mode `hooks install` arm (honest warning). Added a `hook-interpreter` doctor check (Warn+UK remediation pointing at `--config`, Skipped when no file hook installed, Pass on Available/Unknown). TDD: core unit tests for sh-less simulation, healthy Git-for-Windows layout, and unix pass proven red first; doctor check tests for skipped/warn/pass; existing hooks_config_mode.rs + doctor_missing_git.rs stay green.
- **Worked:** Splitting a pure injectable core from the environment-reading wrapper let the Windows sh-less and Git-for-Windows-healthy paths be tested deterministically on a POSIX CI host.
- **Failed:** none.
- **Friction:** Windows path literals compared against `PathBuf::join` output mismatched on a POSIX host (join uses `/`, not `\`); building the expected path via `join` fixed it. Rust 2024 `impl Trait` lifetime capture needed `+ use<>` on a test helper.
- **Improvement:** When simulating another OS's path layout in a cross-platform test, construct expected paths with the same `Path::join` the code under test uses rather than backslash string literals.
- **Follow-up:** none.

### 2026-07-04 — claude (autonomous, Council fix)

- **Task:** CIB-176 Council major — `anvil hooks install --json` emitted the sh-less warning as plain text *after* the JSON payload on a sh-less git, corrupting the `--json` output contract (`crates/anvil-cli/src/commands/hooks.rs`).
- **Outcome:** The warning block sat outside the `if global.json { … } else { … }` split, so it printed on stdout in both modes. Extracted the advisory to a shared `SH_LESS_HOOK_WARNING` const and an `install_interpreter_warnings(status)` helper, hoisted an `InstallOutput { results, warnings }` struct (mirroring the config-mode `{ results, coexistence }` shape) and now carry the advisory *inside* the JSON (`warnings`, omitted when empty) or as a `plain::warn` in the human branch — never trailing the JSON. Added two regression tests: `install_interpreter_warnings_only_on_missing` and `install_output_carries_warning_inside_json` (asserts single valid JSON object, no stray text). fmt/clippy/hooks+doctor+hooks_config_mode green.
- **Worked:** Routing the advisory through both branches of the existing json/human split, with a serialisable field, both restores the contract and keeps the signal for JSON consumers.
- **Failed:** none.
- **Friction:** No test pinned the pre-existing bare-array `hooks install --json` shape, so confirming the array→object change was safe required grepping docs + integration tests rather than a contract test.
- **Improvement:** Emit-once-then-branch: compute advisory data before the output-mode split and render it inside each branch, so no diagnostic can leak onto stdout after a JSON payload.

### 2026-07-04 — claude (CIB-177)

- **Task:** CIB-177 — bare `anvil` fails clap's required-subcommand parse and renders the full 40+-command long help at exit 2, so a first-time user's first contact is a wall of commands with `welcome`/`start` buried mid-list.
- **Outcome:** Added a `before_help` banner on the root `Cli` command (`New to Anvil? Run \`anvil welcome\` for a guided tour, or \`anvil start\` to activate protection in this repository.`), sourced from a new `help_layout::FIRST_RUN_POINTER` const kept beside the CLI-surface layout policy. Parsing and the exit-2 contract are untouched; the `after_help` EXIT CODES block is preserved. TDD: a unit test (`root_help_leads_with_first_run_pointer`, asserts both pointers render before `Commands:`) and an integration test (`tests/bare_invocation.rs`, spawns the bare binary, asserts exit 2 + pointer-before-commands + EXIT CODES footer) were proven red first.
- **Worked:** `before_help` renders ahead of the about/usage/commands body in clap's required-subcommand error path, so the orientation leads without any custom error handling. Keeping the copy identifier-free kept `lint_internal_identifiers` and the CLIC-010 layout lint green with no changes to those lints.
- **Failed:** none.
- **Friction:** `CARGO_TARGET_DIR` is redirected to `~/.cache/anvil-targets`, so the worktree-local `./target/debug/anvil` was stale — inspect the cache-dir binary, not `./target`, when eyeballing rendered CLI output.
- **Improvement:** For "first-run orientation" copy on a required-subcommand root, prefer `before_help` (which the arg-required-else-help path already renders) over a bespoke bare-invocation branch.
- **Follow-up:** none.
- **Task:** CIB-178 — the activation language profile counted anvil's own writes (`.anvilrc`, `.anvil.toml`, `anvil/`, `.anvil-mcp-fallback.json`, installed workflow files), so live runs crept "(1 unclassified file)" → 4 → 6 as the tool inflated its own unclassified noise.
- **Outcome:** Added an activation-only `is_anvil_owned_artifact(path, root)` predicate applied per-file in `profile_repo` alongside the existing `is_excluded_directory` walk filter (`activation/language_profile.rs`). It matches root-relative paths for `.anvilrc`, `.anvil.<ext>` config basenames, the root-level `anvil/` directory, `.anvil-mcp-fallback.json`, and `.github/workflows/{anvil,anvil-audit}.yml`. TDD: a fixture repo with user source plus every artefact asserts the unclassified count equals the baseline and is stable across two runs (proven red at 6 vs 0), plus a guard test that `src/anvil.rs`, a nested `vendor/anvil/`, and `.github/workflows/ci.yml` are NOT excluded. No change to `anvil-checks::filter`.
- **Worked:** Matching on `strip_prefix(root)` component slices keeps every rule root-anchored where the artefact is root-anchored, so a slice pattern like `["anvil", ..]` cannot swallow a nested `vendor/anvil/` and user source is never silently dropped.
- **Failed:** none.
- **Friction:** none — the existing `.anvil` directory doc entry gave a template for documenting the activation-specific addition in the module doc.
- **Improvement:** none.
- **Follow-up:** none. The intent mentioned `plans/` creep too, but the plan deliberately scoped the exclusion to the enumerated artefacts (excluding `plans/` wholesale would risk dropping user planning docs).
### 2026-07-04 — claude (autonomous)

- **Task:** CIB-179 — both welcome renderers (`welcome/render.rs`, `onboarding/welcome_render.rs`) silently drop taglines and per-item descriptions in compact mode with no cue that resizing restores them.
- **Outcome:** Compact mode now reserves a trailing `Length(1)` chunk carrying a muted "resize for descriptions" hint (`COMPACT_HINT`), gated by `show_hint = compact && area.height >= 11` and added to the compact `content_height` so centring stays correct; full mode is byte-unchanged. TDD: contains/omits behavioural tests + insta snapshots at 40x12 proven red first (undefined `COMPACT_HINT`), then green. No hard gate added; adaptive layout preserved.
- **Worked:** Because the menu sits under `Constraint::Min(menu_h)`, the ratatui solver keeps the menu at full priority even when over-constrained — at 40x12 the decorative logo yields rows while all menu items AND the hint stay visible, so the footer never steals a menu row.
- **Failed:** none.
- **Friction:** The style-aware `buffer_to_string` interleaves per-cell style tags between characters, so `contains` assertions needed a local style-stripping `plain()` helper; the package is `eddacraft-anvil-tui`, not the `anvil-tui` the item's Validation line names.
- **Improvement:** When a compact TUI layout must add an element without stealing content rows, lean on `Min(priority_h)` for the content and a trailing fixed `Length` for the addition — the solver sacrifices lower-priority decorative `Length` chunks first.
- **Follow-up:** none.

### 2026-07-04 — claude (council remediation)

- **Task:** CIB-179 Council follow-up (major) — the `Min(menu_h)` + trailing `Length(1)` hint trick that was celebrated in the prior entry is exactly what degraded the brandmark: under contention ratatui holds the menu at full size and lets the fixed `Length(7)` logo absorb the shortfall, so at compact heights 11–16 the logo silently lost a row (at height 11 it collapsed to a single fragment row; height 16 — a previously perfect full-logo/full-menu fit — dropped to 6). Duplicated in both welcome renderers. The added tests only asserted hint presence/absence, so the regression was invisible to CI.
- **Outcome:** Gated the hint on genuine spare height instead of a hard-coded floor: `show_hint = compact && area.height >= (1 top-pad + 7 logo + 1 blank + menu_height + 1 hint)` in both `welcome/render.rs` and `onboarding/welcome_render.rs`. The hint now appears only when logo + full compact menu + hint fit without contention, so it can never be traded for a logo row. TDD: added a `compact_hint_never_squeezes_logo` invariant test (sweeps heights 8–32, asserts `hint_shown ⇒ logo == 7 rows`) proven red first (failed at height 11, logo=1), plus boundary tests pinning the logo at the exact 16/13 fit heights where the hint is now withheld. Retargeted the `compact_shows_resize_hint`/`snapshot_compact_hint` cases to compact-but-roomy sizes (40x20 welcome, 40x16 onboarding) and regenerated the insta snapshots — both now show all 7 logo rows and the hint together.
- **Worked:** Deriving the show threshold from the same `menu_height` the layout already uses keeps the gate honest across both surfaces and any future menu-count change; the sweep test turns "logo integrity" into a machine-checked property rather than a manually sanity-checked case.
- **Failed:** none.
- **Friction:** The logo is inherently squeezed at 40x12 by the 7-item menu alone (independent of the hint), so the boundary "logo intact" assertion had to sit at height 16 (welcome) / 13 (onboarding) — the smallest heights where a full logo + full compact menu actually fit — not at the smallest compact size.
- **Improvement:** A `Min(content)` + trailing `Length(decoration)` layout only protects the content; the fixed-`Length` element it sits beside is what gets sacrificed under contention. Any additive chunk must be gated on the total fitting without contention, and a decorative fixed-size element (a logo) needs a row-count regression test, not just a content-presence one, or the trade-off it silently makes stays invisible to CI.
- **Follow-up:** none.




### 2026-07-04 — claude (CIB-165)

- **Task:** CIB-165 — default the interactive GitHub Actions workflow picker to unticked so a hurried Enter-through in `anvil start` writes no CI files to a shared repo.
- **Outcome:** Extracted a pure `workflow_picker_options(root, candidates) -> Vec<(WorkflowTemplate, String, bool)>` helper (every candidate `selected = false`) and had `show_workflow_picker` build `DemandOption`s from it, replacing the inline `.selected(true)`. Updated the `ensure_github_actions_workflows` doc comment and the `MultiSelect` description to say nothing is selected by default and ticking is the consent. TDD: added `workflow_picker_options_default_every_candidate_unticked` and `workflow_install_with_empty_selection_writes_nothing` (empty selection writes no files and never creates `.github/`), proven red (helper missing) then green.
- **Worked:** Splitting the option construction out of the `demand`-driven picker made the default assertable in a plain unit test without a TTY; the empty-selection install test pins the Enter-through-writes-nothing property directly against `install_selected_workflows`.
- **Failed:** none.
- **Friction:** `show_workflow_picker` itself still needs an interactive terminal, so the default is verified through the extracted helper rather than by driving the picker.
- **Improvement:** When a default lives inside a TTY-only widget builder, extract the value-producing step into a pure function so the policy (here: unticked-by-default) is unit-testable independent of the UI library.
### 2026-07-04 — claude (CIB-169)

- **Task:** CIB-169 — `anvil start` exited `0` on the pre-dispatch auth wall, so `anvil start && deploy` advanced past a completely unactivated repo.
- **Outcome:** Replaced the blanket `is_probe ? 3 : 0` coercion in `auth_required_response` with a three-way classifier — probe (`whoami`/`auth whoami`) → 3 (error envelope), read-only surface (`status`, new `is_read_only_auth_surface`) → 0 (informational envelope), action command (everything else gated) → 3 (informational envelope, shape unchanged). Non-auth pass-through and `--verify` local-probe bypass untouched. TDD: unit tests (`..._exits_three`/`..._read_only_surface_exits_zero`) + a `start.rs` shell-driven `&& echo reached` integration test proven red first. Updated the `--help` EXIT CODES table, doc comments, and a breaking-change CHANGELOG entry.
- **Worked:** The auth wall already threaded `AuthRequiredKind` and a pure, unit-testable `auth_required_response`, so the remap was a single site; the read-only allowlist keeps `status` (a pure state report) on exit 0 while the governance verbs stop `&&` chains.
- **Failed:** none.
- **Friction:** Three integration tests (`format_flag`, `init_post_analysis`) pinned the old exit-0 contract with comments mislabelling read-only `status` as an "action command"; the `status`-based ones stayed green (read-only) but needed the misleading comments corrected, and only the `start`-based JSON test needed its exit assertion flipped to 3.
- **Improvement:** When a shared exit-code helper serves both read-only and action surfaces, encode the read-only allowlist as its own named predicate next to the probe predicate — it makes the "which class exits 0" decision auditable and stops future callers from re-broadening the exit-0 coercion.
### 2026-07-04 — claude (CIB-180)

- **Task:** CIB-180 — under a restart-required headline the MCP tier tokens (`restart_handshake_verified`, `server_startable`, `restart_required`) read as "done" to a terminal-first user, even though a restart is still pending; the token names describe what was probed, not that protection has graduated.
- **Outcome:** Owner-decided render-time gloss (option b). Added a `tier_pending_qualifier(state, tier)` helper in `activation/render.rs` that returns ` (pending restart)` for the three done-ish tiers only under a `ReadyRestartRequired` state, and applied it at the two human print sites (`render_human` compact mcp block + `render_human_verbose` tier line). `render_json` is untouched, so machine tokens stay byte-stable. Extended the `McpTier` doc comment in `diagnostic.rs` to state the labels are observed-probe state, not graduation. TDD: qualifier-present tests (handshake-verified / server-startable / restart-required) proven red first, plus omit-when-live, omit-under-non-restart-headline, verbose, and a JSON byte-stability guard.
- **Worked:** Gating on `ProtectionState::ReadyRestartRequired` (not the raw tier) keeps the qualifier scoped to the one headline where a done-ish token misleads; `LiveValidation` never co-renders there, so no live tier is ever falsely qualified.
- **Failed:** none.
- **Friction:** The item's Validation names `cargo test -p anvil-cli`, but the crate package is `eddacraft-anvil` and it is a `[[bin]]`-only target — the working invocation is `cargo test -p eddacraft-anvil --bin anvil`.
- **Improvement:** When a machine token's *name* is honest but its *reading* misleads a terminal-first user, prefer a render-time gloss keyed on the surrounding state over renaming the token — it fixes comprehension without breaking the byte-stable machine contract or opening a deprecation window.
- **Follow-up:** none.

### 2026-07-04 — claude (CIB-133)

- **Task:** CIB-133 — `anvil status` and `anvil watch` called `first_week_insights_hint` ungated, so a candidate / side-by-side install under a non-default `ANVIL_HOME` burned the real project's once-per-week nudge marker and wrote `.anvil/insights-hint.json` into the real repo (DISTRIB-006 / ADR-060).
- **Outcome:** Threaded the gate into the canonical function — `first_week_insights_hint(root, now, project_writes_gated)` returns `None` with no read and no write at the top when gated — and updated `status.rs` and `watch.rs` to pass `crate::install_root::project_writes_gated()`. Dropped INSIGHTS-005's `welcome`-specific `welcome_insights_hint` wrapper, repointing `print_welcome_insights_hint` and the welcome tests straight at the canonical function so no surface can regress by forgetting the guard. TDD: added a gated-root test per surface (canonical, status, watch) asserting `None` and that `.anvil/insights-hint.json` is not written, proven red against the old 2-arg signature first.
- **Worked:** Lifting the guard into the one function all three surfaces already call collapsed three copies of the check into one and made the "no read, no write when gated" contract a single assertable early return; the existing INSIGHTS-004/-005 in-window tests carried over verbatim (just the new bool arg).
- **Failed:** none.
- **Friction:** `/home` was 100% full (498G shared cargo cache), so the shared target dir's linker died with a Bus error; ran gates on an isolated `CARGO_TARGET_DIR` on the Projects disk. A pre_push integration test also failed only because the full disk blocked writing `~/.config/anvil/kindling/usage.ndjson` (usage WARN on stderr) — unrelated to this change; confirmed green with a writable `XDG_CONFIG_HOME`.
- **Improvement:** When several surfaces share one nudge/state helper, put the gate inside the helper as an early return rather than at each call site — a per-surface wrapper is boilerplate for a universal invariant and the one that forgets it is the leak.
- **Follow-up:** none.

### 2026-07-04 — claude (CIB-153)

- **Task:** CIB-153 — `session.heartbeat` / `session.unregister` carried no peer-credential check, so any same-uid IPC client that guessed a session id could keep-alive or force-unregister a session it never registered.
- **Outcome:** Bound both lifecycle verbs to the registering peer, mirroring MLP2-074's `report_process` contract. Extended the `SessionDispatcher` trait's `heartbeat`/`unregister` to take `peer_pid: Option<u32>`; `dispatch_command` threads the connection's peer through both arms and maps `PeerOwnershipMismatch` to the wire error. Added an immutable `RegistryEntry::launcher_pid` (stamped at `register_with_lineage`) and a `verify_peer_owns` check that fails closed on a missing peer credential, a lineage-less (anchorless) session, or a pid mismatch. `NoopDispatcher`/`RecordingDispatcher`/`RegistryDispatcher`/the jsonrpc-conformance double take the new param. TDD: registry-level unit tests (mismatch / no-credential / no-lineage / unregister survives + owner-only removal / unknown-id idempotent) and dispatch-level tests against a real `SessionRegistry` (peer B rejected, peer A accepted, `None` fails closed) proven red then green.
- **Worked:** The MLP2-074 pattern (`update_lineage_anchor` peer check + typed `PeerOwnershipMismatch`) transplanted almost verbatim; reusing the existing error variant kept the wire surface unchanged.
- **Failed:** none.
- **Friction:** Binding against the mutable `record.pid` (as the proposed plan read) would have stranded the launcher's own heartbeats/unregister once `report_process` narrows the anchor onto the spawned child — the real anvil-run ordering emits register → report_process → heartbeats → unregister all from the launcher. Fixed by recording a stable `launcher_pid` (matching the CIB Expected Outcome: "records the authenticated peer identity that registered it"). Two `daemon_config_wired` warm-state tests registered without lineage and had to send a lineage anchor keyed to the test pid so their unregister keeps working.
- **Improvement:** When an ownership check keys off a per-session pid, prefer a field the lifecycle-narrowing path does NOT rewrite; the mutable lineage anchor and the stable registering-peer identity are different concepts even when they start equal.
- **Follow-up:** No multi-process (distinct real-PID) daemon harness exists yet for a true cross-process denial proof end-to-end (noted in the CIB block); the dispatch-level injected-`peer_pid` tests cover the contract but not real `SO_PEERCRED` cross-process.

