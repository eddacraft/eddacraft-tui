# Post-merge: fix-tuir-004-mirror-workflow-hardening

PR: #NNN
Branch: `fix/tuir-004-mirror-workflow-hardening`
APS: TUIR-004
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Dispatch `mirror-eddacraft-tui.yml` from `main` with
      `dry_run=true` and confirm the run reaches "Force-push to mirror repo",
      emits `::notice::dry_run=true`, and exits 0 without mutating the public
      `eddacraft-tui` mirror. This is the live preflight the hardening adds:
      it validates App auth + refspec + tree shape post-merge. (human required)
- [ ] Step 2 — Dispatch `mirror-acknowledgements-starter.yml` from `main` with
      `dry_run=true` and confirm the same dry-run notice + exit 0 with no push
      to the public acknowledgements-starter mirror. (human required)
- [ ] Step 3 — Confirm both runbook cross-links resolve:
      `docs/runbooks/eddacraft-tui-release.md` and
      `docs/runbooks/acknowledgements-starter-release.md` each contain the
      credential-rotation section the workflow headers now point at.
      (agent: yes)
- [ ] Step 4 — On the next real (non-dry-run) mirror resync, confirm the banner
      double-prepend sentinel does not fire (canonical README has no banner) and
      the produced mirror tree carries exactly one read-only-mirror banner.
      (human required)

## Notes

These workflows force-push to public sibling repos, so the live verification is
deliberately gated behind a manual `workflow_dispatch` dry-run rather than run
automatically. The `dry_run` input was added precisely so the first dispatch
after merge (and after any future credential rotation) can be validated without
mutating public history — exercise it before relying on the live push path.

The CI gate `pnpm test:ci-mirror-hardening`
(`scripts/ci/mirror-workflow-hardening.test.sh`) statically locks all four
guards (pre-delete, dry_run, runbook cross-link, banner sentinel) against
regression; it runs in `ci.yml` and passed pre-merge. The post-merge steps above
cover the parts a static test cannot: live App auth and the real force-push
refspec against the public mirrors.
