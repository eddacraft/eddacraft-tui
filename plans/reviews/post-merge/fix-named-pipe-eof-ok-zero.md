# Post-merge: fix-named-pipe-eof-ok-zero

PR: #2137
Branch: `fix/named-pipe-eof-ok-zero`
APS: CLAWP-004 (`plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Context

`crates/anvil-intercept-win32/src/lib.rs` is `#![cfg(windows)]`, so the changed
code and its round-trip test compile out entirely on the Linux dev/CI box. The
EOF → `Ok(0)` mapping and the post-payload `assert_eq!(eof, 0)` can only be
exercised on a real Windows target. The steps below are the verifications the
in-PR gates cannot run here.

## Steps

- [ ] On a Windows runner, build the crate and run its tests:
      `cargo test -p eddacraft-anvil-intercept-win32` — confirm the round-trip
      test passes and the post-payload read returns `Ok(0)` (the new EOF
      assertion). (agent: yes — only on a Windows runner)
- [ ] On a Windows runner, run `cargo clippy -p eddacraft-anvil-intercept-win32
      --all-targets -- -D warnings` and confirm clean (the win32 paths are only
      linted under `cfg(windows)`). (agent: yes — only on a Windows runner)
- [ ] Confirm `GetLastError() == ERROR_BROKEN_PIPE` (109) is the only failure
      code mapped to `Ok(0)`; every other `ReadFile` failure still returns
      `Err`. Spot-check against a forced non-EOF error in a throwaway branch if a
      regression is suspected. (human required)

## Notes

- This is a behaviour fix for the documented "0 indicates EOF" contract on
  `OwnerOnlyPipeClient::read`. No current Windows call site observes the
  post-payload EOF, so impact is post-tag — but the contract is now pinned by
  the round-trip test.
- Linux gates run for this branch: `cargo fmt -p eddacraft-anvil-intercept-win32
  --check` (green). The crate is a no-op build on Linux.
