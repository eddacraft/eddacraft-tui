# Continuous Improvement Log

This file captures lightweight session learning from agents. It is evidence, not
a backlog. Promote repeated friction or executable follow-up work to
`plans/modules/continuous-improvement-backlog.aps.md` as `CIB-NNN` items.

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
