# Post-merge: feat-opsup-006-guards

PR: #1656
Branch: `feat/opsup-006-guards`
APS: OPSUP-006
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance OPSUP-006 status `In Progress` → `Merged` in `plans/archive/modules/operational-supplement.aps.md` (agent: yes)
- [ ] Update OPSUP module note in `plans/index.aps.md` row to drop the "In Progress (file-presence + wall-time framework)" call-out once status reflects merge (agent: yes)
- [ ] Confirm `cargo test -p eddacraft-anvil --bin anvil 'commands::check_guards'` passes on `main` after merge (agent: yes)
- [ ] Confirm `cargo test -p eddacraft-anvil --bin anvil commands::check_catalog::tests::opsup_006_core_checks_default_to_unguarded` passes on `main` after merge (agent: yes)

## Notes

OPSUP-006 ships the **framework** only — no Track 3 surface or Track 4 pack
check opts in yet. The framework is dormant in the runtime path: every
current core check defaults to `file_shape_globs: &[]` and
`wall_time_soft_budget_secs: None`, so the dispatcher's pre-flight and
post-flight branches are no-ops for them. Migration safety is asserted by
`opsup_006_core_checks_default_to_unguarded` — any regression of that test
on `main` after merge means a downstream change introduced a guard
intent without an OPSUP module update.

Wall-time soft budget is reporting-only — do not advance to a "hard cap"
implementation as a "completion" of this slice. A hard cap requires
cooperative-cancellation refactoring of each underlying check and belongs
in a separate slice with its own design review.

The next OPSUP slice unblocked by this work is OPSUP-005 (per-track
feature flag taxonomy) — surface checks will need both the file-shape
guard and a track flag to ship safely.
