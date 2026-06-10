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
