# Post-merge: feat-dsv-047-supervisor

PR: #3191
Branch: `feat/dsv-047-supervisor`
APS: DSV-047
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [x] `cargo test -p eddacraft-anvil-intercept --lib save_time_driver` green on
      main (20 tests: lifecycle, enqueue-only hook, reconcile sweep,
      shutdown-race, discriminator invariant, dir precedence) — verified
      2026-07-05 on the merged tree (agent: yes)
- [x] `cargo test -p eddacraft-anvil-intercept` full crate green on main — the
      `DaemonLauncher::spawn_detached` signature change touches the ensure
      state machine and its fakes; 1026 passed / 0 failed, verified
      2026-07-05 (agent: yes)
- [ ] Live supervisor smoke — start the daemon via
      `anvil intercept start --foreground` in a licensed environment, durably
      register a worktree (`anvil start`), and confirm: one detached
      `anvil watch --save-time-driver` child appears; `<stem>.pid` +
      `<stem>.spawn.log` land under
      `{ANVIL_HOME}/runtime/save-time-drivers/`; unregistering stops the
      child; `ANVIL_NO_SAVE_TIME_DRIVER=1` suppresses spawning. Needs the beta
      licence (exit 3 unauthenticated); covered structurally by the fake
      launcher/process seams, end-to-end by the DSV-051 E2E matrix
      (human/DSV-051)
- [ ] Windows leg — see `plans/execution/DSV-051.windows.actions.md` §1 and
      §3 (headless spawn discipline; taskkill → `failed`, no auto-respawn);
      runs after DSV-049..050 merge (human required)

## Notes

The supervisor's artefact directory comes from
`save_time_driver::default_driver_dir()` (`{ANVIL_HOME}/runtime/save-time-drivers/`
precedence, matching the child's default log resolution) — NOT the PID-file
parent. Do not "simplify" it back: under `ANVIL_HOME` the PID file skips the
`runtime/` segment and the two paths diverge. A tracked driver on a
start-time-capable platform always carries its `(pid, start_time)`
discriminator; keep that invariant when touching spawn or reconcile. DSV-049
reads `driver_status`/`status_snapshot`; the no-auto-respawn behaviour is a
pinned cut-line decision — a respawn/backoff policy needs its own APS item,
not a drive-by change.
