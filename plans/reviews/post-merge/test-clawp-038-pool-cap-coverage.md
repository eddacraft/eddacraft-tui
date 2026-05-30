# Post-merge: test-clawp-038-pool-cap-coverage

PR: #NNN
Branch: `test/clawp-038-pool-cap-coverage`
APS: CLAWP-038 (module `clawpatch-pre-tag-v0.7.0-beta`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Reconcile CLAWP-038 status to `Merged YYYY-MM-DD via PR #NNN` in
      the clawpatch module and bump the module done/total count in
      `plans/index.aps.md` (agent: yes). This was intentionally NOT done in this
      PR — the CLAWP-038 count cell is shared with sibling branches in the same
      batch and editing it here would cause an `index.aps.md` merge conflict.
- [ ] Step 2 — Confirm the cap-coverage tests run in CI's Rust test job (agent:
      yes): `cargo test -p eddacraft-anvil-rayon-init` shows
      `cap_threads_is_half_cores_minimum_one` and
      `cap_threads_matches_documented_policy_for_current_machine` passing.
- [ ] Step 3 — Close GitHub issue #1651 if not auto-closed by the `Fixes #1651`
      trailer (agent: yes).

## Notes

Scope was coverage-only with a behaviour-preserving refactor: the inline cap
expression `(num_cpus::get() / 2).max(1)` inside `init_global` was extracted to a
pure `cap_threads(available_cores)` helper so the half-cores-minimum-1 policy can
be pinned by a unit test. The global-pool path itself remains untestable in
shared-process unit tests (whichever rayon consumer drives `build_global` first
wins the race), which is exactly why the issue recommended the pure-helper
approach. No runtime behaviour changed — `init_global` still applies the same cap.

Files touched by the code PR:

- `crates/anvil-rayon-init/src/lib.rs` — extracted `cap_threads` helper; added two
  tests pinning the cap policy.

Local gates run green on this branch:

- `cargo fmt -p eddacraft-anvil-rayon-init` + `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p eddacraft-anvil-rayon-init`
