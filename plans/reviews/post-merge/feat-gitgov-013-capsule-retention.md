# Post-merge: feat-gitgov-013-capsule-retention

PR: #TBD
Branch: `feat/gitgov-013-capsule-retention`
APS: GITGOV-013
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] `anvil capsule prune --keep-last 1` in a scratch repo with two staged
      capsules prints the older capsule path and deletes nothing; with
      `--apply` it stages the deletion via the git index (agent: yes —
      `cargo test -p eddacraft-anvil capsule::tests::prune` +
      `cargo test -p eddacraft-anvil-capsule prune`)
- [ ] ADR-078 is Accepted, indexed in DECISION-LOG, and `pnpm adr:check`
      is clean (agent: yes)
- [ ] docs/public/anvil/concepts/review-capsules.md renders the new
      "Retention and pruning" section on the docs site after the next
      docs deploy (human required — visual check)

## Notes

GITGOV-013 closes the GITGOV module's open frontier; GITGOV-001..014 are
now all terminal. The `--json` prune output is explicitly deferred in
ADR-078 until a consumer exists.
