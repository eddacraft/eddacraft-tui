# Post-merge: fix-cib-165-workflow-picker-default-unticked

PR: #NNN
Branch: `fix/cib-165-workflow-picker-default-unticked`
APS: CIB-165
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm `cargo test -p eddacraft-anvil --bins --
      workflow_picker_options_default_every_candidate_unticked
      workflow_install_with_empty_selection_writes_nothing` passes on `main`
      post-merge — both filters run (libtest OR-combines them), so the two new
      tests are exercised (agent: yes)
- [ ] Manual TTY verification (unit tests cover the extracted helper, not the
      rendered `demand::MultiSelect`): in a scratch repo without
      `.github/workflows/anvil*.yml`, run `anvil start` in an interactive
      terminal, reach the "Install or enable GitHub Actions workflows?" picker,
      and confirm both entries render unticked with the "Nothing is selected by
      default" description; press Enter without ticking and confirm no
      `.github/` directory or workflow file is created (agent: no — needs an
      interactive terminal)
- [ ] Reconcile CIB-165 status in
      `plans/modules/continuous-improvement-backlog.aps.md` to
      `Merged YYYY-MM-DD via PR #NNN` (agent: yes)

## Notes

CIB-165 (owner decision 2026-07-04): the interactive workflow picker in
`anvil start` pre-selected both PR validation and Nightly audit, so a hurried
Enter-through silently wrote `.github/workflows/anvil.yml` +
`anvil-audit.yml` — the most repo-visible, PR-triggering write activation
performs.

Fix: extracted a pure `workflow_picker_options(root, candidates)` helper in
`crates/anvil-cli/src/activation/orchestrator/mod.rs` returning
`(workflow, label, selected)` tuples with `selected = false` for every
candidate; `show_workflow_picker` builds its `DemandOption`s from it. A plain
Enter now selects nothing and writes nothing; ticking a workflow is the
explicit consent. Doc comment and picker description updated to match.

The default is unit-tested through the extracted helper because
`show_workflow_picker` requires an interactive terminal — hence the manual TTY
step above.
