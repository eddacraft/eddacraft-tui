# Post-merge: claude-next-jrel-dependency-0e6bqa

PR: #4428
Branch: `claude/next-jrel-dependency-0e6bqa`
APS: JREL-003, JREL-004
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

### JREL-004 — one daemon identity through start and recycle

- [ ] XDG unset then set: with `XDG_RUNTIME_DIR` unset run `anvil start`; then
      from a shell with it set run bare `anvil`. Expect `daemon: running`, one
      PID file, and `anvil intercept status` naming the state-home socket with
      the same PID (human required)
- [ ] XDG set then unset (needs a real `/run/user/<uid>`): reverse the order.
      Expect reuse through the implicit sibling and no second daemon (human
      required)
- [ ] Concurrent starts from mixed shells: `for i in 1 2 3 4; do anvil & done;
      wait`. Expect exactly one `daemon: started` (human required)
- [ ] Stale canonical plus live sibling: `kill -9` a runtime-dir daemon, start
      one from a plain shell, then bare `anvil` from the XDG shell. Expect
      reuse, status showing the live PID, no duplicate (human required)
- [ ] Concurrent recycle: with an older-version daemon running, run two
      `anvil start` concurrently. Expect one `recycled (old -> new)`, the other
      `running`, and the replacement PID untouched (human required)
- [ ] Isolated homes: `ANVIL_HOME=/tmp/a anvil start` and
      `ANVIL_HOME=/tmp/b anvil`. Expect distinct PIDs; stopping one leaves the
      other (human required)
- [ ] `anvil doctor --fix` with a sibling daemon still repairs without hanging
      (human required)
- [ ] `cargo test -p eddacraft-anvil --test daemon_identity -- --test-threads=1`
      passes on merged `main` (agent: yes)

### JREL-003 — recover and verify the save-time driver

- [ ] `anvil start --no-mcp` in a small repository; `anvil intercept status
      --json` shows `save_time_driver: "attached"` with
      `save_time_driver_evidence: "watches-installed"` (human required)
- [ ] `kill -9` the PID in the driver's `<stem>.pid` under the runtime
      `save-time-drivers` directory; status now reports `failed`, not
      `attached` (human required)
- [ ] Run bare `anvil`: expect `worktree: registration refreshed`, a new PID in
      `<stem>.pid`, exactly one `anvil watch --save-time-driver` process for the
      worktree, and status `attached` (human required)
- [ ] Save a `.ts` file containing `: any`; within a few seconds `<stem>.log`
      shows the AP-003 finding and status reports `fresh-activity` (human
      required)
- [ ] Bound: make the anvil binary unexecutable, run `anvil` four times; the
      spawn log shows three attempts then a refused respawn and the `worktree:`
      line names the failed driver. Restore the binary, wait 60 s, run `anvil`
      once and expect `attached` again (human required)
- [ ] macOS: repeat the kill and re-run steps (zombie reaping is Unix-generic);
      Windows: confirm `anvil intercept status` is unchanged and the evidence key
      is absent unless the child wrote the marker (human required)
- [ ] `cargo test -p eddacraft-anvil --test save_time_driver_recovery` passes on
      merged `main` (agent: yes)

## Notes

The JREL-004 closeout was validated in a Linux sandbox running as uid 0, so
the XDG-set-then-unset leg and the chmod-based fixtures need a normal user
account. Both crate suites otherwise passed; see the module closeout in
`../../archive/modules/journey-reliability.aps.md`.
