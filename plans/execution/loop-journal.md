# APS Loop Journal

One entry per loop cycle. Resume point for interrupted runs and the
operator's audit trail. Bookkeeping only — committed with plan changes,
never with feature work.

## Cycle 1 — 2026-06-10

- Item: UJ-005 — `anvil status` always states the save-time posture
- Outcome: done (validation: `cargo test -p eddacraft-anvil
  commands::status` 52 green incl. 6 new posture tests; workspace clippy
  `-D warnings` clean; fresh-context outcome verification PASS;
  quick-tier council review no blocking findings, both MINOR findings
  addressed). PR #2500, auto-merge armed. Manual transcript blocked by
  the intentional beta licence gate — unit tests carry the rendering
  validation.
- Plan changes: UJ-005 → Merged 2026-06-10 via PR #2500; UJ module +
  index counts 1/11 (script-managed).
- Checkpoints raised: none
- Next: UJ-006 (watch help/advisory daemon guidance) — independent
  files; bookkeeping flips deferred until #2500 lands to avoid index
  count-cell collisions.

## Cycle 2 — 2026-06-10

- Item: UJ-006 — Daemon guidance on the watch surface and help
- Outcome: done (validation: `cargo test -p eddacraft-anvil
  commands::watch` 75 green incl. 2 new tests — long-help assertion via
  the real `Cli::command()` surface + fallback-advisory pointer;
  workspace clippy clean; `anvil watch --help` transcript shows the
  Save-time daemon section; fresh-context verification CONDITIONAL PASS
  — both MAJOR findings fixed: misattached doc-comment split, `=1`
  wording corrected to match the warned-fallback implementation; ASCII
  advisory per the watch banner policy). PR #2501.
- Plan changes: UJ-006 → Merged 2026-06-10 via PR #2501; UJ counts 2/11
  (script-managed).
- Checkpoints raised: none
- Next: UJ-001 (golden-path next-step threading in install/welcome/
  init/start).

## Cycle 3 — 2026-06-10

- Item: UJ-001 — Golden-path next-step threading in CLI and install output
- Outcome: done (validation: 6 new closing-hint tests green — start ×3
  incl. restart_required honesty case, init, welcome ×2; full crate
  suite green; workspace clippy + fmt clean; `bash -n install.sh`
  clean; fresh-context verification PASS — all four surfaces' exit
  paths traced, live-claim-at-restart_required finding fixed with a
  pinning test). PR #2502.
- Plan changes: UJ-001 → Merged 2026-06-10 via PR #2502; UJ counts 3/11
  (script-managed).
- Checkpoints raised: none
- Next: UJ-003 (entry docs rewritten around the two paths) — in flight
  in a parallel worktree.

## Cycle 4 — 2026-06-10

- Item: UJ-003 — Quickstart and beta guide rewritten around the two paths
- Outcome: done (validation: `pnpm docs:check` 8/8 surfaces green;
  `pnpm docs:index` + oxfmt clean; fresh-context verification
  CONDITIONAL PASS — both findings fixed: "in every routing state"
  save-time claim corrected for the =0 opt-out, ANVIL_HOME note
  re-worded to platform-standard paths incl. Windows %APPDATA%. Drift
  corrected: item said `anvil mcp-install`; real surface is `anvil
  start` wiring + `anvil mcp-config --target`; quickstart documents the
  real commands and the module item carries the correction note.)
  PR #2503.
- Plan changes: UJ-003 → Merged 2026-06-10 via PR #2503; UJ counts 4/11
  (script-managed); UJ-003 body annotated with the mcp-install drift
  correction.
- Checkpoints raised: none
- Next: UJ-008 (consolidated save-time validation guide).

## Cycle 5 — 2026-06-10

- Item: UJ-008 — Consolidated save-time validation guide
- Outcome: done (validation: `pnpm docs:check` 8/8; docs:index + oxfmt
  clean; new guide carries governance + Upstream/Downstream tables and
  the sidebar entry; fresh-context fact-check FAIL→fixed: nonexistent
  `anvil workspace remove` corrected to `deny` (also a pre-existing
  config.md bug), nested-backtick span, `--foreground` requiredness,
  reconnect-rescan guarantee softened to best-effort, Windows parity
  claim narrowed to daemon-status correlation). PR #2504.
- Plan changes: UJ-008 → Merged 2026-06-10 via PR #2504; UJ counts
  (script-managed).
- Checkpoints raised: none
- Next: UJ-002 validation + UJ-009.

## Cycle 6 — 2026-06-10

- Item: UJ-002 — Welcome path lands on a populated surface
- Outcome: done, no change needed (validation: code trace — welcome hub
  options all gather live data at launch; first-run lands on the
  discovery-scan-populated tutorial; no welcome surface recommends a
  persisted-data target. Item body carries the disposition.)
- Plan changes: UJ-002 → Done 2026-06-10 (verified — no change needed).
- Checkpoints raised: none
- Next: UJ-009 (gate-summary reach) — implemented in a parallel
  worktree, PR to follow.

## Cycle 7 — 2026-06-10

- Item: UJ-009 — Gate-summary dashboard reaches existing projects
- Outcome: done (chosen path: zero-write built-in fallback — the picker
  and `anvil dashboard gate-summary` serve the embedded
  GATE_SUMMARY_SPEC whenever no saved spec shadows it; saved specs
  always win; embedded spec stays the CIB-053 single source. Validation:
  4 new tests green + full anvil/anvil-tui suites; clippy+fmt clean;
  real transcripts in a fresh repo — picker --json lists the builtin
  entry, direct launch resolves where it previously bailed. Subagent
  verification unavailable (spend limit); all six surfaces traced
  inline for shadowing consistency. Accepted edge: outside any
  workspace the direct launch falls through to the unknown-name error.)
  PR #2505.
- Plan changes: UJ-009 → Merged 2026-06-10 via PR #2505; UJ counts 7/11
  (script-managed).
- Checkpoints raised: none
- Next: UJ-010 (PR #2506 open, flips deferred); then only design-gated
  items (UJ-004, UJ-007) and UJ-011 shaping remain.

## Cycle 8 — 2026-06-10

- Item: UJ-010 — Post-upgrade what's-new one-liner
- Outcome: done (validation: 5 new `whats_new` tests green — seed-
  silently, same-version silent, announce-with-pointer, exactly-once
  across runs, suppressed/gated runs write nothing; full anvil +
  anvil-tui suites green; workspace clippy + fmt clean. Manual
  transcript blocked by the licence gate; scope note: the line carries
  version + changelog pointer, a fetched release headline would need
  the network probe and was deliberately left out.) PR #2506.
- Plan changes: UJ-010 → Merged 2026-06-10 via PR #2506; UJ counts 8/11
  (script-managed); UJ-011 annotated with the shaping draft
  (`plans/execution/UJ-011.shaping.md`); lessons recorded in
  `plans/execution/lessons/2026-06-10-uj-loop-run.md`.
- Checkpoints raised:
  - UJ-004 (auth-wall placement vs first wow): needs a new ADR — every
    manual transcript this run hit the login wall before any value,
    exactly the failure the item names; `dashboard` is already ungated
    and demonstrates the alternative. Proposal for the operator: ungate
    read-only `anvil welcome` as the demo surface, keep durable
    surfaces (`init`, `start`, `watch`) gated.
  - UJ-007 (watch offers to start the daemon): UJ-001/UJ-006 shipped
    "run `anvil start`" guidance on every relevant surface; the item
    itself names guidance-only as an acceptable resolution. Proposal:
    affirm guidance-only (no prompt, no auto-start) as an ADR-075
    amendment and close UJ-007.
  - UJ-011: shaping draft ready for operator review (see above);
    follow-up items UJ-012..015 file on approval.
- Next: stop — no Ready items remain; everything left is
  checkpoint-level (UJ-004, UJ-007, UJ-011 review).

## Cycle 9 — 2026-06-10

- Item: UJ-009 follow-up — seeded gate-summary stem shadows the builtin
- Outcome: done (a project memory note surfaced a shipped bug in
  #2505 minutes after merge: `spec::discover` names saved specs by file
  stem, so the init-seeded spec is `gate-summary.dashboard`, which the
  exact-name shadow check missed — seeded projects double-listed the
  dashboard and the direct launch served the embedded copy over the
  saved spec. Fix: `.dashboard`-suffix-aware shadow check + saved-spec
  precedence at both launch dispatch points. Validation: new
  red→green test, 29 dashboard tests green, clippy+fmt clean, real
  transcript on a simulated seeded project — single listing, direct
  launch resolves to the saved spec.) PR #2507.
- Plan changes: none (fix to already-Merged UJ-009; no status change).
- Checkpoints raised: none new (UJ-004 / UJ-007 / UJ-011 review stand
  from cycle 8).
- Next: stop — no Ready items remain; UJ is 8/11 with three
  checkpoint-level items (UJ-004 ADR, UJ-007 decision, UJ-011 shaping
  review) awaiting the operator.

## Cycle 10 — 2026-06-10

- Item: UJ-007 + UJ-011 — operator decisions executed
- Outcome: done (operator: "guidance-only" for UJ-007 → ADR-079
  Accepted, zero code; UJ-011 shaping approved with both open questions
  answered — fold ci.md into the GitHub guide, align web tutorials with
  the in-terminal `anvil tutorial` narrative). UJ-004 ("ungate
  welcome") executes next as ADR-080 + the CLI_GATED_COMMANDS change.
- Plan changes: UJ-007 → Done (ADR-079); UJ-011 → Done (shaping
  approved); UJ-012..015 filed as Ready (tutorial execution set);
  DECISION-LOG row for ADR-079; shaping doc marked approved.
- Checkpoints raised: none — all three cycle-8 checkpoints now have
  operator answers.
- Next: UJ-004 (ADR-080 + ungate `welcome`), then UJ-012..015 are the
  remaining Ready work.

## Cycle 11 — 2026-06-10

- Item: UJ-004 — Auth-wall placement vs first wow
- Outcome: done (operator: "ungate welcome" → ADR-080 Accepted +
  `welcome` removed from CLI_GATED_COMMANDS. Validation: TDD red→green
  on `requires_auth_welcome` (now asserts NOT gated) +
  `command_needs_licence_gate_rejects_bypass_commands` pins the
  exclusion; full suite, clippy, fmt green; real transcript —
  unauthenticated `anvil welcome` renders the surface ending in the
  `anvil start` handoff, where it previously printed the
  refresh-token error. ADR records the accepted consequence that
  welcome's guided setup can seed config without auth.) PR #2509.
- Plan changes: UJ-004 → Merged 2026-06-10 via PR #2509; UJ counts
  11/15; DECISION-LOG row for ADR-080.
- Checkpoints raised: none — all operator decisions executed.
- Next: UJ-012..015 (tutorial execution set) are the module's remaining
  Ready work.

## Cycle 12 — 2026-06-10

- Item: UJ-012 — "Your first save caught" tutorial
- Outcome: done (new flagship tutorial
  `docs/public/anvil/tutorials/first-save-caught.md` walking
  `anvil start` → daemon (`anvil intercept start --foreground`) →
  `anvil watch --source` → deliberate bad save → reading the finding →
  `anvil status` posture, narrative-aligned with the in-terminal
  ProtectionLoop path; linked from the quickstart daily-value step and
  the tutorials index. Validation: `pnpm docs:check` 8/8,
  `pnpm docs:index` clean, `format:check` exit 0; fresh-context
  verification confirmed every transcript snippet against the rendering
  code and caught one overclaim — `anvil start` does not launch the
  daemon (activation orchestrator excludes daemon spawn by design) —
  fixed before PR. Live end-to-end walk blocked by the agent
  environment's licence gate + inotify limits; gate-unavailable is not
  a content veto.) PR #2510.
- Plan changes: UJ-012 → Merged 2026-06-10 via PR #2510; UJ counts
  12/15; stale "UJ is 10/15" release-plan row note reconciled.
- Checkpoints raised: none.
- Next: UJ-013 (Rust analysis tutorial) — Ready, independent of UJ-012.

## Cycle 13 — 2026-06-10

- Item: UJ-013 — "Analyse a Rust project" tutorial
- Outcome: done (new tutorial
  `docs/public/anvil/tutorials/rust-project.md`: discovery via
  `anvil welcome` / `anvil check --all`, RS-001..005 advisory catalogue
  with the cfg(test)-exclusion scope stated per rule, suppression
  syntax, the `anvil start --verify` language-profile claim, ending on
  the daily-value handoff. Listed in the tutorials index. Validation:
  docs:check 8/8, docs:index clean, format:check exit 0; fresh-context
  verification caught two real inaccuracies pre-PR — test exclusion is
  RS-001/002-only, and the sample output was missing the message line —
  both fixed.) PR #2511.
- Plan changes: UJ-013 → Merged 2026-06-10 via PR #2511; UJ counts
  13/15.
- Checkpoints raised: none.
- Next: UJ-014 (tutorial refresh + index rewrite) — architecture/drift
  legs already drafted in a parallel worktree; index rewrite unblocks
  once #2511 merges.

## Cycle 14 — 2026-06-10

- Item: UJ-014 — Refresh surviving tutorials + journey-aligned index
- Outcome: done (architecture.md gains the Rust architecture.yaml
  example, resolution-semantics note, and Rust fix example; its
  misleading Suppress block — `@anvil-ignore ARCH-001` is not wired to
  boundary violations — replaced with depends_on edits + the
  new-edges-only baseline posture; drift.md cross-links the
  dashboard/insights guides; index.md rewritten around the two beta
  paths, naming the in-terminal `anvil tutorial` as the interactive
  sibling, listing the final five-tutorial set with guide pointers
  replacing the ci/suppressions rows. Validation: docs:check 8/8,
  docs:index clean, format:check exit 0; two fresh-context reviews —
  the suppression doc bug was their catch.) PR #2513.
- Plan changes: UJ-014 → Merged 2026-06-10 via PR #2513; UJ counts
  14/15.
- Checkpoints raised: none.
- Next: UJ-015 (retire ci/suppressions into their guides) — unblocked
  once #2513 merges. Pre-verification already done: suppressions.md is
  heavily drifted (wrong suppressions.json schema vs the shipped
  loader, no SUP-001 rule, comma multi-rule unsupported by the ADR-029
  regex) — the fold carries only code-verified content.

## Cycle 15 — 2026-06-10

- Item: UJ-015 — Retire ci and suppressions tutorials into their guides
- Outcome: done (folds: github.md gains matrix/auth-tip/ci-profile/
  other-CI-systems; insights.md gains "Suppressing a finding" with the
  code-verified inline syntax; git-hooks.md gains the staged-only
  pre-commit example. Removals + sweep: both tutorial pages deleted;
  quickstart/first-project/git-hooks/git-hook-compatibility repointed;
  BOTH docs apps' manual sidebars updated — the sweep also added the
  UJ-012/UJ-013 pages that were missing from those explicit lists; the
  docs-check baseline regenerated canonically. Deliberately not
  carried: the old file-level/bulk-suppression sections (drift —
  suppressions.json is export/dashboard-only, schema mismatched the
  shipped loader), SUP-001 (nonexistent), comma multi-rule directives
  (unsupported). Validation: docs:check 8/8, docs:index clean,
  format:check exit 0, Docusaurus build SUCCESS; fresh-context review's
  two real findings fixed pre-PR, its third resolved by #2513's merge.)
  PR #2514.
- Plan changes: UJ-015 → Merged 2026-06-10 via PR #2514; UJ counts
  15/15; module status → Complete (archive deferred to release-tag
  inclusion per archiving rules).
- Checkpoints raised: none.
- Next: stop — the UJ module is fully dispositioned; no Ready items
  remain in the UJ-012..015 scope this run was invoked for.
