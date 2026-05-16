# Post-merge: feat/mlp2-016-real-engine

PR: <!-- filled when opened -->
Branch: `feat/mlp2-016-real-engine`
APS: MLP2-016 (reopened by 2026-05-15 Council audit)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance MLP2-016 status `In Progress` → `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Update MLP2 progress count in `plans/index.aps.md`
      (48/68 → 49/68 once -016 is back to Merged) (agent: yes)
- [ ] File a follow-up APS task for "ship `patterns/compiled/registry.json`
      with installed `anvil` binary" — without it, the engine still
      degrades to `EngineUnavailable { BinaryMissing }` on a stock
      install. The hook surfaces a `ValidationPending` line so the
      degradation is no longer silent, but the operator still cannot
      enforce L4 rules until the registry is on disk. (human required —
      needs a new MLP2-NNN id allocated)
- [ ] File a follow-up APS task for `git cat-file --batch` integration to
      cut the per-blob `Command::spawn` cost on large commits (current
      cost ~5-15ms × N files; MLP2-022 budget is 2s). (human required)
- [ ] Optional: add `EngineUnavailableReason::IoError` to `anvil-l4` so
      `TempDir` and similar I/O failures stop overloading
      `BinaryMissing`. (human required)

## Notes

- The engine is wired and the e2e test
  (`production_default_engine_blocks_known_antipattern` in
  `crates/anvil-cli/src/commands/l4_validate.rs`) exercises the
  production default constructor (`default_engine()`) without injecting
  a fixture engine. This satisfies the audit's load-bearing
  requirement.
- Council quick (3 personas: adversarial-reviewer, kernel-maintainer,
  operations-reviewer) ran pre-PR. CRITICALs and MAJORs addressed in
  the same PR; remaining MINORs are tracked above as follow-ups.
- The pre-push hook now emits a `tracing::warn!` carrying the
  `EngineUnavailable` reason (Council #C-016I) so a production
  incident has a machine-readable signal distinguishing the failure
  modes that previously all collapsed to one stderr line.
- Delete-only commits intentionally admit — antipattern rules detect
  code being introduced. `--diff-filter=ACMR` makes the intent
  explicit at the git-plumbing surface and a test pins the behaviour.
