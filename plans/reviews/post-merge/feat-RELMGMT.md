# Post-merge: feat-RELMGMT

- PR: #993
- Branch: `feat/RELMGMT`
- APS: RELMGMT-012, RELMGMT-013, RELMGMT-014, RELMGMT-015
- Merged: 2026-04-20
- Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] `./scripts/release.sh` runs to completion on a clean tree and exits 0
      (agent: yes)
- [ ] `./scripts/release.sh` reports per-step failures without short-circuiting
      when an induced failure is added (agent: yes)
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
by running the script locally. The last three require a real release cycle and
should be checked off the next time `/release` is used; they are the true
validation of the Phase 3 rework (agent-driven, resumable, manifest-free).

Module RELMGMT is already marked Complete at 15/15 in
`plans/modules/release-management.aps.md` and `plans/index.aps.md`. No further
APS status updates required from this merge.
