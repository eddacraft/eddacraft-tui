# Post-merge: feat-actmo-durable-registration

PR: #NNN
Branch: `feat/actmo-durable-registration`
APS: ACTMO (ACTMO-014..018)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

These are the cut-line gate scenarios (design test matrix) that need a live
daemon + real worktrees, so they cannot run in unit tests. Verify manually on a
box where `anvil intercept` can bind (not the inotify-exhausted agent shell).

- [ ] Durability — `anvil workspace register`, let the process exit, wait > 30 s,
      `anvil status`: worktree still registered (agent: no — needs wall clock)
- [ ] Restart recovery — register, `anvil intercept stop` then `anvil start`:
      registered set reloaded; daemon log shows
      `"registered worktrees on startup"` (agent: no)
- [ ] Reaper — register a worktree, `git worktree remove` it, wait for the
      60 s sweep (or restart): dropped + reported (agent: no)
- [ ] Cap — register past the distinct-worktree cap (default 64): clear
      `RegisteredWorktreeCapExceeded` message (agent: no — needs 65 worktrees)
- [ ] Different spelling/symlink — register the same worktree via a symlink:
      `Refreshed`, not an error (ADR-094 decision 3) (agent: no)
- [ ] Outside-worktree start — `anvil start` in `$HOME` (non-worktree): exit 0,
      daemon ensured, "no worktree registered" guidance, cwd not registered
      (agent: yes — exit code + stdout assertion)
- [ ] `register --all` over a mixed allowlist: only exact live unfenced entries
      registered; prefix/fenced/gone skipped + reported; no filesystem scan
      (agent: no — needs an allowlist fixture + daemon)
- [ ] `intercept stop` with N registered: prints the count guidance; daemon
      logs one INFO with the count (agent: no)
- [ ] Windows named-pipe parity for register/list/unregister; display paths free
      of `\\?\` via dunce (agent: no — needs Windows; cross-check only)

## Notes

- Keystone commit `7708c2b0b` (daemon) carries the registry/persistence/reload
  work; `61145c639` the CLI surface; `84339cf91` the status surfacing.
- The protecting/watching assurance axis (ACTMO-017) is deliberately a soft-dep
  on DSV-046 and is NOT asserted by this PR — only the membership axis ships.
- DSV-046's headless driver consumes the new registry membership-change signal
  (`SessionRegistry::set_membership_hook`); that wiring lands with DSV-046.
- Item statuses stay **In Progress** here; flip ACTMO-014..018 to
  `Merged YYYY-MM-DD via PR #NNN` after merge (do not bump the module `N/M`
  header — ADR-053 advisory counts).
