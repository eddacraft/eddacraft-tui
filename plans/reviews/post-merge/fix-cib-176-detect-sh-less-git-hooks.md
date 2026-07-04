# Post-merge: fix-cib-176-detect-sh-less-git-hooks

PR: #3149
Branch: `fix/cib-176-detect-sh-less-git-hooks`
APS: CIB-176
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm CIB-176 is `Merged 2026-07-04 via PR #3149` in
      `plans/modules/continuous-improvement-backlog.aps.md` (agent: yes)
- [ ] Windows sh-less verification: on a Git for Windows install with the
      bundled `usr/bin/sh.exe` removed or renamed (and no `sh.exe` on PATH),
      run `anvil hooks install` and confirm the sh-less advisory prints (human
      or JSON mode), then `anvil doctor` shows the `hook-interpreter` check as
      Warn with the `--config` remediation (human required — Windows box)
- [ ] Windows healthy-path verification: on a standard Git for Windows
      install, confirm `anvil hooks install` emits no interpreter warning and
      `anvil doctor` reports `hook-interpreter` as Pass (human required —
      Windows box)
- [ ] `anvil hooks install --json` on the sh-less box emits a single valid
      JSON object with the advisory inside `warnings` and no trailing text
      (human required — Windows box)

## Notes

The interpreter probe core (`detect_hook_interpreter`) is fully unit-tested
cross-platform by simulating the Windows layouts on POSIX, so these steps only
confirm the environment-reading wrapper against a real sh-less Git for
Windows. The unix path needs no post-merge action. If no sh-less Windows box
is available, dispatching `rust.yml` on a branch that renames the bundled
`sh.exe` in a setup step is an acceptable substitute.
