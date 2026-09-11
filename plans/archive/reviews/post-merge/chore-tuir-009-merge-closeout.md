# Post-merge: chore-tuir-009-merge-closeout

PR: #2339
Branch: `chore/tuir-009-merge-closeout`
APS: TUIR-009
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] anvil-001 `releases/latest` returns `v0.7.4-beta` (the Anvil
      product release), not `eddacraft-tui-v0.2.4` — `gh api
      repos/eddacraft/anvil-001/releases/latest --jq .tag_name`
      (agent: yes)
- [ ] eddacraft-tui mirror `releases/latest` returns
      `eddacraft-tui-v0.2.4` (the new prefixed tag), not the prior
      legacy `v0.2.2` — `gh api
      repos/eddacraft/eddacraft-tui/releases/latest --jq .tag_name`
      (agent: yes)
- [ ] anvil-001 v0.2.3 and v0.2.4 crate releases are flagged
      `prerelease: true` — `gh api
      repos/eddacraft/anvil-001/releases/tags/eddacraft-tui-v0.2.3
      --jq .prerelease` and same for v0.2.4 (agent: yes)
- [ ] Mirror `eddacraft-tui-v0.2.4` release `target_commitish` equals
      tag commit `39a95550c09964595fabdbb8f06d02fb92fefcf8` (not
      `main` HEAD) — `gh api
      repos/eddacraft/eddacraft-tui/releases/tags/eddacraft-tui-v0.2.4
      --jq .target_commitish` (agent: yes)
- [ ] anvil-001 `eddacraft-tui-v0.2.4` release `target_commitish`
      equals tag commit `39a95550c09964595fabdbb8f06d02fb92fefcf8` —
      `gh api
      repos/eddacraft/anvil-001/releases/tags/eddacraft-tui-v0.2.4
      --jq .target_commitish` (agent: yes)
- [ ] Mirror `releases/latest` body matches the local CHANGELOG
      v0.2.4 entry — `gh api -H 'Accept: application/vnd.github.raw'
      repos/eddacraft/eddacraft-tui/contents/CHANGELOG.md` byte-diff
      against `crates/eddacraft-tui/CHANGELOG.md` (agent: yes)
- [ ] Mirror README body matches the local README.md (offset derived
      from `wc -l < MIRROR-README.md`) — `gh api -H 'Accept:
      application/vnd.github.raw'
      repos/eddacraft/eddacraft-tui/contents/README.md | tail -n
      +$((MIRROR_README_BANNER_LINES + 1))` byte-diff against
      `crates/eddacraft-tui/README.md` (agent: yes)
- [ ] Next `eddacraft-tui-vX.Y.Z` cut follows the updated runbook
      (operator-driven, next time TUIR-005 publish workflow fires).
      Verify by re-reading
      `docs/runbooks/eddacraft-tui-release.md` step 8: must use
      `--prerelease` (agent: yes)
- [ ] Re-validate `pnpm docs:check` and `cargo check -p eddacraft-tui
      --features json-render` on the merged main to confirm the
      runbook's `MIRROR_README_BANNER_LINES` derivation still
      resolves to a non-empty offset (agent: yes)
- [ ] TUIR-008 execution-token (live E2E cut of `eddacraft-tui-v0.2.3`
      on crates.io, mirror tag propagation, legacy
      `EDDACRAFT_TUI_MIRROR_PUSH_TOKEN` PAT revocation, downstream
      consumer check against private `eddacraft/eddacraft-skills`) is
      still `open` and unblocked by this work — verify the
      `docs/runbooks/eddacraft-tui-release.md` "Cutover & history
      rewrite" and "Two-layer migration rollback" sections still
      reflect the post-TUIR-009 workflow changes (agent: yes)

## Notes

- TUIR-009's structural fix lands in PR #2339, squash commit
  `817b359b11f06f0d7dbe5d3886f6a62b169c61e6`. All operational mirror +
  anvil-001 release flag changes were applied manually on 2026-06-07
  (before the PR) so PR #2339 ratifies the *workflow* change
  (`--prerelease`) and the *docs* change (runbook + APS); the live
  state is already correct on both repos.
- The mirror's CHANGELOG/README byte-diff will report drift
  immediately after PR #2339 merges, because the local README has
  the new "Why eddacraft-tui over vanilla `ratatui`?" comparison
  table that hasn't been pushed to the mirror yet. The expected
  follow-up is the next `mirror-eddacraft-tui.yml` run (triggered by
  the next push to `crates/eddacraft-tui/**` on main), which will
  close the drift. The verify step is *expected to fail* until the
  next mirror sync lands; log the result, don't block the closeout.
- `gh release create --prerelease` semantics: per GitHub REST API,
  the release is excluded from `…/releases/latest` when
  `prerelease: true`. The anvil-001 `releases/latest` should
  therefore surface the most recent *non-prerelease* release, which
  is the latest Anvil product release (e.g. `v0.7.4-beta`).
- The Copilot review fix (`190cfd3e5`) replaced a hardcoded
  `tail -n +45` with `wc -l < MIRROR-README.md` so the offset
  survives banner growth; the offset is currently 44 (44 banner
  lines → canonical body starts at line 45).
