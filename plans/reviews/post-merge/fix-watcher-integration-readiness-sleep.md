# Post-merge: fix-watcher-integration-readiness-sleep

PR: #NNN
Branch: `fix/watcher-integration-readiness-sleep`
APS: CLAWP-033 (clawpatch-pre-tag-v0.7.0-beta)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm `filters_out_non_parseable_files` no longer flakes on CI across a
      few runs after merge — watch the next handful of `anvil-kernel` test runs
      on `main` for any recurrence of the empty/stale-batch timeout (agent: no —
      requires observing post-merge CI history)
- [ ] Consider the Council follow-up: extract a shared watcher-test helper that
      uses one conservative readiness strategy for both
      `detects_parseable_file_creation` and `filters_out_non_parseable_files`,
      or wait on a sentinel parseable file before asserting, instead of a fixed
      warm-up sleep. This change only aligned the two warm-up budgets (250 ms)
      as the minimal fix. (agent: no — design follow-up, file a CIB if pursued)

## Notes

This was the minimal, low-risk fix for the CLAWP-033 finding: the flaky test's
OS-watcher warm-up sleep was 50 ms while its sibling already used a conservative
250 ms; on a loaded runner the 50 ms budget could expire before watch
registration landed, so `recv_timeout(2s)` returned a stale/empty batch and the
test flaked. The fix bumps the warm-up to 250 ms to match the sibling.

The follow-up step (shared helper / sentinel-file readiness) is the Council
reviewer's longer-term recommendation, deliberately left out of this PR to keep
it single-purpose. If pursued, raise it as a CIB item rather than reopening this
branch.
