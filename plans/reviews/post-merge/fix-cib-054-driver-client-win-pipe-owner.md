# Post-merge: fix-cib-054-driver-client-win-pipe-owner

PR: #2485
Branch: `fix/cib-054-driver-client-win-pipe-owner`
APS: CIB-056
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Windows-native gate verification — on a Windows host with the daemon
      running, `DriverClient.connect()` with the daemon-logged pipe name
      succeeds, and a pipe name carrying a different SID suffix rejects with
      `anvil-daemon-wrong-owner` (human required — no Windows box in the
      agent environment; fold into the Windows CI matrix follow-up, INTD-012
      successor)
- [ ] Confirm issue #2484 stays open after merge with only the pipe-SD /
      server-identity increment remaining, and its body reflects that
      increment 1 (SID-suffix gate) shipped via PR #2485 (agent: yes)
- [ ] Flip clawpatch finding `fnd_sig-feat-cli-command-a4f9ddbd8c-_55d076e44e`
      local status to fixed once increment 1 is on main
      (`clawpatch triage --finding … --status fixed`) — note that #2484
      still tracks the deferred pipe-squat defence (agent: yes)

## Notes

The fix is client-side defence-in-depth: the daemon-side owner-only DACL +
client-SID check (DSV-010b / ADR-070) is the authoritative server gate and is
unchanged. The new client gate is fully unit-tested on Linux via the
`currentUserSid` injection seam (219 package tests, 23 in windows.test.ts);
only the real `whoami.exe` resolution path needs a Windows host to observe
end-to-end.
