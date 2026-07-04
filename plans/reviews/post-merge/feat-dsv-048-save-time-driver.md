# Post-merge: feat-dsv-048-save-time-driver

PR: #3186
Branch: `feat/dsv-048-save-time-driver`
APS: DSV-048
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] `cargo test -p eddacraft-anvil --bin anvil -- watch_save_time_driver` green
      on main (10 tests) (agent: yes)
- [ ] `anvil watch --save-time-driver` without `--worktree` exits with a clap
      usage error, and `anvil watch --save-time-driver --worktree . --action
      gate` exits with a conflict error (`--worktree` supplied so the check
      exercises the conflict, not the missing-required path) (agent: yes)
- [ ] Live-daemon verdict-to-log smoke — a planted antipattern finding lands in
      the `ANVIL_SAVE_TIME_DRIVER_LOG` file within one debounce window. Needs a
      licensed environment (beta auth wall, exit 3 unauthenticated); covered
      structurally by unit tests, end-to-end by the DSV-051 E2E matrix
      (human/DSV-051)
- [ ] Windows leg — see `plans/execution/DSV-051.windows.actions.md` §1–2; runs
      after DSV-047..050 merge (human required)

## Notes

The driver child owns the findings log (single-writer; supervisor redirects
stdout/stderr to a separate `.spawn.log`). Rotation is remove-then-rename so a
pre-existing `.1` never wedges appends on Windows. The `--exclude` conflict
exists to keep bare-exclude advisories off the headless stdout contract —
do not relax the conflict set without re-checking that path. DSV-047 consumes
the argv contract pinned here; do not change flag names/requirements without a
paired supervisor change.
