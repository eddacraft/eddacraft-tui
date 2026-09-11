# Post-merge: feat-distrib-003-homebrew-formula

PR: #1652
Branch: `feat/distrib-003-homebrew-formula`
APS: DISTRIB-003
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] On the next release tag (e.g. `v0.7.0-beta`), confirm `release.yml`'s
      "Publish Homebrew formula" step calls
      `scripts/release/bump-homebrew.sh` and the patched formula reaches
      `eddacraft/homebrew-tap` as `Formula/anvil.rb`. (agent: yes — `gh api
      repos/eddacraft/homebrew-tap/commits?path=Formula/anvil.rb --jq
      '.[0].commit.message'` should print `anvil <tag>`.)
- [ ] On the same release, confirm the `Homebrew — bump and smoke` workflow's
      `smoke` matrix passed on macOS arm64 (`macos-14`) and x64 (`macos-13`).
      (agent: yes — `gh run list --workflow homebrew-bump.yml --branch main
      --limit 5 --json conclusion,name,headSha`.)
- [ ] Operator: validate the `workflow_dispatch` recovery path manually at
      least once against an older tag with `skip-publish=true` to confirm
      the dry-run + smoke surface runs without writing to the tap.
      (human required)
- [ ] Operator: confirm `ANVIL_RELEASES_TOKEN` has `contents:write` on
      `eddacraft/homebrew-tap` per `docs/runbooks/release-token-scope.md`.
      The script will exit 78 with a clear message if the token is missing
      locally, but a misscoped token only surfaces as a 403 inside the
      `gh api PUT`. (human required, one-time per token rotation.)
- [ ] Advance APS `DISTRIB-003` from `Merged` to `Released/Shipped` when
      the first release using this script lands. (agent: yes — invoked by
      `scripts/aps-cleanup.sh` if present.)
- [ ] On `Released/Shipped`, advance the module progress counter in
      `plans/archive/modules/distribution-and-update.aps.md` and
      `plans/index.aps.md` from 3/5 to 4/5. (agent: yes.)

## Notes

The DISTRIB-003 implementation is intentionally a refactor + new surface
rather than new release-pipeline behaviour — the inline Homebrew publish
step in `release.yml` was already auto-bumping the tap. What changed:

- The publish logic is now testable (`scripts/release/_test/bump-homebrew.test.sh`).
- The dry-run runs on every PR that touches the script or workflow.
- A `workflow_dispatch` recovery surface and a macOS arm64/x64 smoke matrix
  exist where there were none.
- The operator runbook documents three recovery paths.

Known gap (called out in the runbook, not in scope for DISTRIB-003): the
publish commit on the tap is unsigned at the git layer. The binary it
points at is minisign-signed per ADR-045 / DISTRIB-001. Tap-commit signing
waits on a release-bot identity with a managed key — track separately.
