# Post-merge: test-clawp-026-baseline-symbol-collision

PR: #NNN
Branch: `test/clawp-026-baseline-symbol-collision`
APS: CLAWP-026 (clawpatch test-hardening finding — no GH issue)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Summary

CLAWP-026 flagged that the baseline-suppression tests in
`crates/anvil-kernel/tests/architecture_parity.rs`
(`previously_public_symbol_suppressed`, `previously_privileged_symbol_suppressed`,
`baseline_suppresses_known_but_flags_new`) never exercised same-name symbol
collisions, leaving a test gap behind a possible real correctness bug (a
name-only baseline key would silently suppress a brand-new symbol that merely
shares a name with a baselined one).

Investigation found the production bug was **already fixed on main** by commit
`6b5307e50 fix(kernel): distinguish baseline symbol identities` (2026-05-19),
which changed `GraphDelta::symbol_baseline_key` to `file::kind::name` and added
two collision regression tests (`same_name_different_file_public_symbol_still_flags_new_export`,
`same_name_different_file_privileged_symbol_still_flags_new_access`).

This PR closes the remaining sub-gap: the existing collision tests use a
single-symbol delta with a hand-built baseline set. They do not exercise the
**mixed single delta** where the baselined original and the new same-name symbol
are BOTH evaluated together. Added `same_name_collision_in_single_delta_flags_only_new_file`
to cover that variant. No production change was required — the fixed key already
handles it (confirmed: a temporary revert to a name-only key turns the new test
red, proving it guards the regression).

`foundRealBug = false` — the underlying correctness bug was already fixed on main;
this is regression-coverage hardening only.

## Steps

- [ ] Reconcile CLAWP-026 status in `plans/index.aps.md` / clawpatch module (agent: parent reconciles — count cell shared with sibling branches) (human required)

## Notes

- Pure-code branch: only the test file and this post-merge doc are touched.
- Do not flip CLAWP-026 status from this branch; the parent reconciles the
  shared count cell after merge per APS policy.
- Gates run green locally: `cargo fmt --all --check`,
  `cargo clippy -p eddacraft-anvil-kernel --all-targets -- -D warnings`,
  `cargo test -p eddacraft-anvil-kernel` (all suites pass; architecture_parity
  25 tests).
