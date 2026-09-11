# Post-merge: feat-actmo-020-install-hook

PR: #3002
Branch: `feat/actmo-020-install-hook`
APS: ACTMO-020
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Mark ACTMO-020 `Merged YYYY-MM-DD via PR #3002` in
      `plans/modules/activation-mcp-optional.aps.md` (agent: yes)
- [ ] End-to-end on a real box: `anvil workspace install-hook`, then `git wt-add
      ../tmp-wt` in a repo, confirm the new worktree appears in `anvil workspace
      list` / `anvil intercept status` (human required — needs a live daemon)
- [ ] Windows parity: `install-hook` on Git-for-Windows runs the alias via its
      bundled `sh`; confirm `git wt-add` registers, and the printed PowerShell
      `$PROFILE` function works as an alternative (human required — Windows box or
      dispatch `rust.yml` on the branch)

## Notes

- ACTMO-020 ships only the `anvil workspace install-hook` git-alias core. The
  Worktrunk post-create hook template is descoped (no confirmed Worktrunk
  post-create hook surface) and deferred to a future item.
- The alias path detection takes the FIRST operand of `git worktree add`
  (skipping flags + the `-b`/`-B` branch value), fixing the ADR-094 D7 draft's
  last-positional form which mis-registered a trailing commit-ish. The trailing
  `f "$@"` arg-forwarding is load-bearing (Git runs `!`-aliases as
  `sh -c '<body>' <name> <args>`). Both are covered by a unit test that runs the
  body in a real `sh` with stubbed `git`/`anvil` — re-verify on the merged binary.
- Depends on ACTMO-019 only for the shared `anvil workspace` surface; functionally
  independent (install-hook calls the existing `anvil workspace register`).
