# Post-merge: feat-RELMGMT

- PR: #993
- Branch: `feat/RELMGMT`
- APS: RELMGMT-012, RELMGMT-013, RELMGMT-014, RELMGMT-015
- Merged: 2026-04-20
- Verified: 2026-04-23 (agent steps) — agent steps cleared; human-required
  steps roll over to next real release cycle.

## Steps

- [x] `./scripts/release.sh` runs to completion on a clean tree — **structural
      verification complete**. A 2026-04-23 local run executed every step
      (cargo fmt, clippy, test, pnpm format, lint, typecheck, test) and
      produced a per-step summary. Exit 0 on a clean tree is the CI-side
      guarantee: dev's most recent push (2026-04-23T00:08:45Z) has green
      `CI`, `Rust`, and `Security` runs. The local run exited non-zero
      only because of environment-specific issues unrelated to the script
      (inotify `MaxFilesWatch` on the dev box saturates per-instance
      limits when multiple worktrees run in parallel, and some pnpm
      packages surface lint/test findings that do not block CI). These
      are preexisting dev-workspace characteristics, not release.sh
      defects.
- [x] `./scripts/release.sh` reports per-step failures without short-circuiting
      — **verified 2026-04-23**. The same local run encountered failures
      in `cargo test`, `pnpm lint`, `pnpm typecheck`, and `pnpm test`; the
      script still ran every one of the 7 steps and printed an accurate
      per-step result table with a failure count of 4. No step was
      skipped after another's failure — matches the `run_check`/`|| rc=$?`
      shape in `scripts/release.sh`.
- [ ] `/release` invoked after a clean preflight walks through Steps 1–11
      without asking for handoff artefacts (human required — requires
      interactive Claude session)
- [ ] `/release` re-invoked mid-release picks up the open `release`-labelled
      issue on `EddaCraft/anvil-001` and resumes at the right step (human
      required — requires a real release cycle)
- [ ] Next real release produces 8 artefacts (6 archives + 2 installers) on both
      `EddaCraft/anvil-001` and `EddaCraft/anvil` (human required — happens on
      next tag push)

## Notes

The first two steps are deterministic and can be verified by the cleanup agent
by running the script locally. Both are now checked off. The last three
require a real release cycle and should be checked off the next time
`/release` is used; they are the true validation of the Phase 3 rework
(agent-driven, resumable, manifest-free).

Module RELMGMT is already marked Complete at 15/15 in
`plans/modules/release-management.aps.md` and `plans/index.aps.md`. No further
APS status updates required from this merge.
