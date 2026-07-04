# Post-merge: fix-cib-178-exclude-anvil-artefacts-language-profile

PR: <!-- filled after PR creation -->
Branch: `fix/cib-178-exclude-anvil-artefacts-language-profile`
APS: CIB-178 (module `continuous-improvement-backlog`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Reconcile CIB-178 status from `In Progress` to
      `Merged YYYY-MM-DD via PR #NNNN` in
      `plans/modules/continuous-improvement-backlog.aps.md` (agent: yes). This
      was intentionally NOT done in this PR — CIB status flips collide on the
      shared module/index count cells when sibling CIB branches merge in the
      same window (ADR-053 advisory counts; reconcile with `pnpm aps:index`
      only if a refresh is needed).
- [ ] Step 2 — Confirm the new tests run in CI's Rust test job (agent: yes):
      `cargo test -p eddacraft-anvil language_profile` shows
      `anvil_owned_artefacts_are_excluded_and_stable_across_runs` and
      `user_files_resembling_anvil_artefacts_are_not_excluded` passing.
- [ ] Step 3 — Live-run sanity check (agent: yes, needs a scratch repo): in a
      fresh non-anvil repo, run activation twice and confirm the reported
      language profile's unclassified count does not grow between run 1 and
      run 2 (previously crept 1 → 4 → 6 as activation wrote `.anvilrc`,
      `.anvil.toml`, `anvil/`, `.anvil-mcp-fallback.json`, and the installed
      workflow files).

## Notes

Activation-only exclusion: a new `is_anvil_owned_artifact(path, root)`
predicate is applied per-file in `profile_repo` alongside the existing
`is_excluded_directory` walk filter. Matching is on root-relative path
component slices, so every rule stays root-anchored where the artefact is
root-anchored — `src/anvil.rs`, a nested `vendor/anvil/`, and
`.github/workflows/ci.yml` are deliberately NOT excluded (guard test pins
this). `anvil-checks::filter` is untouched; scan/check behaviour is
unchanged.

Files touched by the code PR:

- `crates/anvil-cli/src/activation/language_profile.rs` — predicate + wiring
  + 2 tests.
- `plans/modules/continuous-improvement-backlog.aps.md` — CIB-178 → In
  Progress.
- `plans/reviews/continuous-improvement-log.md` — CI log entry.

Local gates run green on this branch:

- `cargo fmt -p eddacraft-anvil --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p eddacraft-anvil language_profile` (20 passed, 0 failed)
