# Post-merge: test-clawp-027-status-render-fixture-race

PR: #2145
Branch: `test/clawp-027-status-render-fixture-race`
APS: CLAWP-027 (module `clawpatch-pre-tag-v0.7.0-beta`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Summary

`crates/anvil-cli/tests/status_render.rs` fixture (re)generation was racy under
the default multi-threaded test harness: under `ANVIL_UPDATE_FIXTURES=1`,
writer test functions and reader test functions touched the same fixture paths
concurrently, so a reader could observe a torn or momentarily-absent file. The
fix routes every fixture file access (read AND write) through a process-wide
`fixture_io_lock()` mutex so reads can never interleave with an in-progress
write. No product behaviour changed — this is test-harness only and adds no new
dependency.

## Steps

- [ ] Verify fixtures unchanged by the fix (agent: yes)
  `git diff <merge-base>..HEAD -- crates/anvil-cli/tests/fixtures/` is empty.
- [ ] Verify suite is green in verify mode (agent: yes)
  `cargo test -p eddacraft-anvil --test status_render` — 5 passed.
- [ ] Verify update mode is deterministic and leaves fixtures byte-identical
  (agent: yes)
  `ANVIL_UPDATE_FIXTURES=1 cargo test -p eddacraft-anvil --test status_render`
  passes, then `git status --porcelain crates/anvil-cli/tests/fixtures/` is
  empty.
- [ ] Confirm CLAWP-027 status reconciled by the parent batch (human required)
  This PR intentionally does NOT touch `plans/index.aps.md` or the clawpatch
  module — the CLAWP-027 count cell is shared with sibling branches in this
  batch and the parent reconciles status after merge. Confirm CLAWP-027 was
  flipped to `Merged YYYY-MM-DD via PR #NNN` post-merge.

## Notes

- The regression test `fixture_io_is_serialised_against_concurrent_readers_and_writers`
  hammers a temp path from 32 threads x 16 write/read cycles through the same
  guarded primitives. It was proven to fail deterministically (5/5 runs, torn
  "EOF while parsing" reads) when the lock guards are removed, and pass with
  them present.
- The test drives the lock primitives directly on a temp path and never sets
  the process-wide `ANVIL_UPDATE_FIXTURES` env var, so it neither perturbs
  sibling tests nor runs only in update mode.
- Gates run green locally: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p eddacraft-anvil --test status_render` (both modes).
